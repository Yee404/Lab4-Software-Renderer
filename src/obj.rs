use crate::vertex::Vertex;
use raylib::math::{Vector2, Vector3};
use tobj;

/// Una pieza (objeto `o` del OBJ) ya expandida a lista de triángulos.
pub struct ObjPart {
    pub name: String,
    pub vertices: Vec<Vertex>,
}

/// Carga un OBJ conservando sus objetos por separado (así cada pieza puede tener su
/// propio color) y centra TODO el modelo una sola vez: el centro de la caja envolvente
/// de todos los vértices queda en el origen, de modo que `ship_position` es el centro
/// real de la nave y rota sobre sí misma.
pub fn load_centered_parts(path: &str) -> Result<Vec<ObjPart>, tobj::LoadError> {
    let (models, _materials) = tobj::load_obj(path, &tobj::GPU_LOAD_OPTIONS)?;

    let mut parts: Vec<ObjPart> = Vec::new();
    for model in models {
        let mesh = &model.mesh;
        let mut vertices = Vec::with_capacity(mesh.indices.len());
        for &index in &mesh.indices {
            let i = index as usize;
            let position = Vector3::new(
                mesh.positions[i * 3],
                mesh.positions[i * 3 + 1],
                mesh.positions[i * 3 + 2],
            );
            let normal = if mesh.normals.is_empty() {
                Vector3::zero()
            } else {
                Vector3::new(mesh.normals[i * 3], mesh.normals[i * 3 + 1], mesh.normals[i * 3 + 2])
            };
            let tex_coords = if mesh.texcoords.is_empty() {
                Vector2::zero()
            } else {
                Vector2::new(mesh.texcoords[i * 2], mesh.texcoords[i * 2 + 1])
            };
            vertices.push(Vertex::new(position, normal, tex_coords));
        }
        parts.push(ObjPart { name: model.name, vertices });
    }

    // Centro real del modelo a partir de los límites de todos sus vértices.
    let mut min = Vector3::new(f32::MAX, f32::MAX, f32::MAX);
    let mut max = Vector3::new(f32::MIN, f32::MIN, f32::MIN);
    for part in &parts {
        for v in &part.vertices {
            min.x = min.x.min(v.position.x);
            min.y = min.y.min(v.position.y);
            min.z = min.z.min(v.position.z);
            max.x = max.x.max(v.position.x);
            max.y = max.y.max(v.position.y);
            max.z = max.z.max(v.position.z);
        }
    }
    let center = (min + max) * 0.5;
    for part in parts.iter_mut() {
        for v in part.vertices.iter_mut() {
            v.position -= center;
            v.transformed_position = v.position;
        }
    }
    Ok(parts)
}
