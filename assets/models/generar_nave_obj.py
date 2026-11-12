"""
Genera la nave estilo Astroneer (low-poly, sombreado plano) directamente como .obj,
sin necesitar Blender.   Uso:   python generar_nave_obj.py

Convenciones del OBJ resultante (las que espera el renderizador):
  - X = derecha, Y = arriba, la PROA apunta a -Z.
  - Cada pieza es un objeto `o <nombre>`; main.rs elige el color según el nombre:
      casco, cabina, modulo, tobera, aleta, antena, pata, luz
  - La nave mide ~10 unidades de largo (se escala con SHIP_SCALE en main.rs).
"""
import math
import os

PARTES = []  # (nombre, [triángulos])


def esfera(cx, cy, cz, rx, ry, rz, seg=12, anillos=8):
    """Elipsoide: lista de triángulos."""
    def p(i, j):
        lat = -math.pi / 2 + math.pi * j / anillos
        lon = 2 * math.pi * i / seg
        return (
            cx + rx * math.cos(lat) * math.cos(lon),
            cy + ry * math.sin(lat),
            cz + rz * math.cos(lat) * math.sin(lon),
        )
    tris = []
    for j in range(anillos):
        for i in range(seg):
            a, b, c, d = p(i, j), p(i, j + 1), p(i + 1, j + 1), p(i + 1, j)
            if j != 0:
                tris.append((a, b, d))
            if j != anillos - 1:
                tris.append((d, b, c))
    return tris


def tronco_z(cx, cy, z0, r0, z1, r1, seg=12):
    """Cilindro / cono truncado a lo largo del eje Z, con tapas."""
    def anillo(z, r, i):
        a = 2 * math.pi * i / seg
        return (cx + r * math.cos(a), cy + r * math.sin(a), z)
    tris = []
    for i in range(seg):
        j = (i + 1) % seg
        a0, a1 = anillo(z0, r0, i), anillo(z0, r0, j)
        b0, b1 = anillo(z1, r1, i), anillo(z1, r1, j)
        tris.append((a0, a1, b0))
        tris.append((a1, b1, b0))
        tris.append(((cx, cy, z0), a1, a0))
        tris.append(((cx, cy, z1), b0, b1))
    return tris


def tronco_y(cx, y0, cz, r0, y1, r1, seg=8):
    """Cilindro vertical (patas, antena)."""
    def anillo(y, r, i):
        a = 2 * math.pi * i / seg
        return (cx + r * math.cos(a), y, cz + r * math.sin(a))
    tris = []
    for i in range(seg):
        j = (i + 1) % seg
        a0, a1 = anillo(y0, r0, i), anillo(y0, r0, j)
        b0, b1 = anillo(y1, r1, i), anillo(y1, r1, j)
        tris.append((a0, a1, b0))
        tris.append((a1, b1, b0))
        tris.append(((cx, y0, cz), a1, a0))
        tris.append(((cx, y1, cz), b0, b1))
    return tris


def caja(x0, x1, y0, y1, z0, z1):
    v = [(x, y, z) for x in (x0, x1) for y in (y0, y1) for z in (z0, z1)]
    # índices: x*4 + y*2 + z
    caras = [
        (0, 1, 3, 2), (4, 6, 7, 5),  # x0, x1
        (0, 4, 5, 1), (2, 3, 7, 6),  # y0, y1
        (0, 2, 6, 4), (1, 5, 7, 3),  # z0, z1
    ]
    tris = []
    for a, b, c, d in caras:
        tris.append((v[a], v[b], v[c]))
        tris.append((v[a], v[c], v[d]))
    return tris


def agregar(nombre, tris):
    PARTES.append((nombre, tris))


# ---------------------------------------------------------------- diseño de la nave
# Casco: cápsula redondeada con una sonda en la proa
agregar("casco", esfera(0, 0, 0, 1.9, 1.4, 3.6))
agregar("casco_sonda", tronco_z(0, -0.1, -3.2, 0.5, -4.7, 0.16, 8))

# Cabina: burbuja sobre la proa
agregar("cabina", esfera(0, 0.95, -1.5, 1.15, 0.95, 1.5, 10, 6))

for lado in (-1, 1):
    x = 3.0 * lado
    n = "izq" if lado < 0 else "der"
    # Módulos laterales (cilindros acoplados) con tapa redondeada en la proa
    agregar("modulo_" + n, tronco_z(x, -0.1, -2.2, 0.85, 2.4, 0.85, 12))
    agregar("modulo_tapa_" + n, esfera(x, -0.1, -2.2, 0.85, 0.85, 0.7, 10, 6))
    # Puntales que unen el módulo con el casco
    agregar("modulo_puntal_" + n,
            caja(min(1.5 * lado, 2.3 * lado), max(1.5 * lado, 2.3 * lado), -0.35, 0.15, -0.9, 0.9))
    # Toberas traseras
    agregar("tobera_" + n, tronco_z(x, -0.1, 2.4, 0.62, 3.7, 1.0, 12))
    # Luces delanteras
    agregar("luz_" + n, esfera(x, -0.1, -2.95, 0.24, 0.24, 0.24, 8, 5))

# Aleta superior y antena
agregar("aleta", caja(-0.14, 0.14, 1.1, 2.7, 1.6, 3.3))
agregar("antena", tronco_y(0.0, 1.3, 0.3, 0.07, 2.7, 0.04, 6))
agregar("luz_antena", esfera(0.0, 2.75, 0.3, 0.14, 0.14, 0.14, 6, 4))

# Patas de aterrizaje
for lado in (-1, 1):
    for z in (-1.4, 1.7):
        x = 1.35 * lado
        agregar("pata", tronco_y(x, -2.3, z, 0.16, -1.0, 0.16, 6))
        agregar("pata_pie", caja(x - 0.38, x + 0.38, -2.45, -2.25, z - 0.38, z + 0.38))

# ---------------------------------------------------------------- escritura del OBJ


def sub(a, b):
    return (a[0] - b[0], a[1] - b[1], a[2] - b[2])


def cruz(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0])


def norm(v):
    l = math.sqrt(v[0] ** 2 + v[1] ** 2 + v[2] ** 2)
    return (v[0] / l, v[1] / l, v[2] / l) if l > 1e-12 else (0.0, 1.0, 0.0)


def escribir(ruta):
    lineas = ["# Nave estilo Astroneer generada por generar_nave_obj.py", ""]
    iv = 1  # índice de vértice (1-based)
    inorm = 1
    total = 0
    for nombre, tris in PARTES:
        # centro de la pieza, para orientar las normales hacia afuera
        pts = [p for t in tris for p in t]
        c = tuple(sum(p[k] for p in pts) / len(pts) for k in range(3))
        lineas.append("o " + nombre)
        for a, b, d in tris:
            n = norm(cruz(sub(b, a), sub(d, a)))
            centro_tri = tuple((a[k] + b[k] + d[k]) / 3 for k in range(3))
            if sum(n[k] * (centro_tri[k] - c[k]) for k in range(3)) < 0:
                n = (-n[0], -n[1], -n[2])
                b, d = d, b  # mantener el sentido de giro coherente con la normal
            for p in (a, b, d):
                lineas.append("v %.5f %.5f %.5f" % p)
            lineas.append("vn %.5f %.5f %.5f" % n)
            lineas.append("f %d//%d %d//%d %d//%d" % (iv, inorm, iv + 1, inorm, iv + 2, inorm))
            iv += 3
            inorm += 1
            total += 1
    with open(ruta, "w", encoding="utf-8") as f:
        f.write("\n".join(lineas) + "\n")
    print("Escrito", ruta, "-", len(PARTES), "piezas,", total, "triángulos")


if __name__ == "__main__":
    escribir(os.path.join(os.path.dirname(os.path.abspath(__file__)), "nave_astroneer.obj"))
