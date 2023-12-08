mod line_duplicates;
mod line_face_connections;
mod line_faces;
mod line_id;
mod manifold_lines;

pub use line_duplicates::*;
pub use line_face_connections::*;
pub use line_faces::*;
pub use line_id::*;
pub use manifold_lines::*;
use usage::Usage;

use std::collections::BTreeMap;

use crate::{
    face::{FaceIndices, FaceLines},
    Vector3, EPSILON,
};

#[derive(Debug, Copy, Clone)]
pub struct Line {
    pub i0: usize,
    pub i1: usize,
}

pub enum LinesTag {}
pub type Lines = Usage<LinesTag, BTreeMap<LineId, Line>>;

pub fn lines(face_indices: &FaceIndices) -> (Lines, FaceLines) {
    let mut line_head = 0;

    let mut face_lines = FaceLines::default();
    let mut lines = Lines::default();

    for (face_id, indices) in face_indices.iter() {
        if indices.len() < 2 {
            continue;
        }

        for i in 0..indices.len() - 1 {
            let line_id = LineId(line_head);
            line_head += 1;

            lines.insert(
                line_id,
                Line {
                    i0: indices[i],
                    i1: indices[i + 1],
                },
            );
            face_lines.entry(*face_id).or_default().push(line_id);
        }

        let line_id = LineId(line_head);
        line_head += 1;

        lines.insert(
            line_id,
            Line {
                i0: indices[indices.len() - 1],
                i1: indices[0],
            },
        );
        face_lines.entry(*face_id).or_default().push(line_id);
    }

    (lines, face_lines)
}

fn line_eq(a0: &Vector3, a1: &Vector3, b0: &Vector3, b1: &Vector3) -> bool {
    if (a0 - b0).magnitude() < EPSILON && (a1 - b1).magnitude() < EPSILON {
        true
    } else if (a0 - b1).magnitude() < EPSILON && (a1 - b0).magnitude() < EPSILON {
        true
    } else {
        false
    }
}

fn point_in_line(point: &Vector3, v0: &Vector3, v1: &Vector3) -> bool {
    let dc = point - v0;
    let dl = v1 - v0;

    let cross = dc.cross(&dl);
    if cross.magnitude() > EPSILON {
        return false
    }

    let dp = dc.dot(point);
    let d0 = dc.dot(v0);
    let d1 = dc.dot(v1);

    dp >= d0 - EPSILON && dp <= d1 + EPSILON
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_in_line() {
        let point = Vector3::new(0.0, 0.0, 0.0);
        let v0 = Vector3::new(1.0, 1.0, 1.0);
        let v1 = Vector3::new(-1.0, -1.0, -1.0);
        let contained = point_in_line(&point, &v0, &v1);
        println!("Contained: {contained:?}");

        let point = Vector3::new(0.0001, 0.0, 0.0);
        let contained = point_in_line(&point, &v0, &v1);
        println!("Contained: {contained:?}");
    }
}
