"""
Genera una nave low-poly estilo Astroneer y la exporta como .obj para el renderizador.

USO (Blender 3.x / 4.x):
  1. Abre Blender -> pestaña "Scripting" -> "Open" -> elige este archivo -> "Run Script" (Alt+P).
  2. Se crea la nave y se exporta a  ~/Downloads/nave_astroneer.obj
     (cambia OUTPUT_PATH abajo si quieres otra ruta).
  3. Copia ese .obj a  assets/models/  (o reemplaza nave.obj) y ajusta main.rs:
        SHIP_YAW_OFFSET = 0.0     (la proa queda en -Z del OBJ, como espera el renderizador)
        SHIP_SCALE      = 0.16    (la nave mide ~10 unidades de largo; 10 * 0.16 = 1.6)

Convenciones: en Blender la proa apunta a +Y. Con el exportador (Forward = -Z, Up = Y) eso
queda como proa = -Z en el OBJ. La nave es simétrica y está centrada.
El renderizador pinta la nave de un solo color, así que aquí no se usan materiales.
"""
import math
import os

import bpy
import bmesh

OUTPUT_PATH = os.path.join(os.path.expanduser("~"), "Downloads", "nave_astroneer.obj")


def limpiar_escena():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()


def registrar(partes, escala=(1, 1, 1)):
    """Toma el último objeto creado, le aplica la escala y lo guarda en la lista."""
    obj = bpy.context.active_object
    obj.scale = escala
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    partes.append(obj)
    return obj


def esfera(partes, pos, escala, segmentos=12, anillos=8):
    bpy.ops.mesh.primitive_uv_sphere_add(
        segments=segmentos, ring_count=anillos, radius=1.0, location=pos
    )
    return registrar(partes, escala)


def cilindro_y(partes, pos, radio, largo, vertices=10):
    """Cilindro alineado con el eje Y (la dirección de la proa)."""
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices,
        radius=radio,
        depth=largo,
        location=pos,
        rotation=(math.radians(90), 0, 0),
    )
    return registrar(partes)


def cilindro_z(partes, pos, radio, largo, vertices=8):
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices, radius=radio, depth=largo, location=pos
    )
    return registrar(partes)


def tobera(partes, pos, radio_unido, radio_salida, largo, vertices=10):
    """Cono truncado que se ensancha hacia atrás (-Y)."""
    bpy.ops.mesh.primitive_cone_add(
        vertices=vertices,
        radius1=radio_unido,
        radius2=radio_salida,
        depth=largo,
        location=pos,
        rotation=(math.radians(90), 0, 0),
    )
    return registrar(partes)


def caja(partes, pos, escala):
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=pos)
    return registrar(partes, escala)


def construir_nave():
    partes = []

    # --- Casco central: cápsula redondeada ---
    esfera(partes, (0, 0, 0), (2.0, 3.8, 1.5))

    # --- Cabina: burbuja sobre la proa ---
    esfera(partes, (0, 1.7, 1.0), (1.25, 1.5, 0.95))

    # --- Módulos laterales (estilo Astroneer: cilindros acoplados) ---
    for lado in (-1, 1):
        x = 3.1 * lado
        cilindro_y(partes, (x, -0.4, -0.1), 0.95, 4.4)
        # Tapa redondeada del módulo, en la proa
        esfera(partes, (x, 1.8, -0.1), (0.95, 0.8, 0.95), segmentos=10, anillos=6)
        # Tobera trasera
        tobera(partes, (x, -3.1, -0.1), 0.75, 1.15, 1.4)
        # Puntal que une el módulo con el casco
        caja(partes, (1.85 * lado, -0.4, -0.1), (1.6, 1.0, 0.35))

    # --- Aleta superior ---
    caja(partes, (0, -2.5, 1.6), (0.25, 1.6, 1.1))

    # --- Antena ---
    cilindro_z(partes, (0, -1.4, 1.95), 0.09, 1.2, vertices=6)

    # --- Patas de aterrizaje ---
    for lado in (-1, 1):
        for y in (1.0, -1.6):
            cilindro_z(partes, (1.3 * lado, y, -1.55), 0.16, 1.1, vertices=6)
            caja(partes, (1.3 * lado, y, -2.15), (0.7, 0.7, 0.12))

    # --- Unir todo en un solo objeto ---
    bpy.ops.object.select_all(action="DESELECT")
    for p in partes:
        p.select_set(True)
    bpy.context.view_layer.objects.active = partes[0]
    bpy.ops.object.join()
    nave = bpy.context.active_object
    nave.name = "nave_astroneer"

    # Triangular y dejar sombreado plano (aspecto low-poly)
    malla = nave.data
    bm = bmesh.new()
    bm.from_mesh(malla)
    bmesh.ops.triangulate(bm, faces=bm.faces[:])
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces[:])
    bm.to_mesh(malla)
    bm.free()
    for poligono in malla.polygons:
        poligono.use_smooth = False

    # Centrar en el origen (el renderizador vuelve a centrar por la caja envolvente)
    bpy.ops.object.origin_set(type="ORIGIN_GEOMETRY", center="BOUNDS")
    nave.location = (0, 0, 0)
    return nave


def exportar(nave, ruta):
    bpy.ops.object.select_all(action="DESELECT")
    nave.select_set(True)
    bpy.context.view_layer.objects.active = nave
    try:
        # Blender 3.2+ / 4.x
        bpy.ops.wm.obj_export(
            filepath=ruta,
            export_selected_objects=True,
            export_materials=False,
            export_uv=True,
            export_normals=True,
            export_triangulated_mesh=True,
            forward_axis="NEGATIVE_Z",
            up_axis="Y",
        )
    except AttributeError:
        # Blender 2.8x - 3.1
        bpy.ops.export_scene.obj(
            filepath=ruta,
            use_selection=True,
            use_materials=False,
            use_uvs=True,
            use_normals=True,
            use_triangles=True,
            axis_forward="-Z",
            axis_up="Y",
        )
    print("Nave exportada en:", ruta)


limpiar_escena()
nave = construir_nave()
print("Triángulos:", len(nave.data.polygons))
exportar(nave, OUTPUT_PATH)
