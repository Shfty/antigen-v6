use std::collections::BTreeSet;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use usage::Usage;

use super::{FaceId, FaceVertices};
use crate::{FacePlanes, EPSILON};

pub enum FaceDuplicatesTag {}

/// The set of opposing faces that share the same set of vertices
pub type FaceDuplicates = Usage<FaceDuplicatesTag, BTreeSet<(FaceId, FaceId)>>;

pub fn face_duplicates(
    planes: &Vec<FaceId>,
    face_planes: &FacePlanes,
    face_vertices: &FaceVertices,
) -> FaceDuplicates {
    planes
        .par_iter()
        .flat_map(|lhs_id| {
            let lhs_verts = &face_vertices[&lhs_id];
            let lhs_plane = &face_planes[&lhs_id];

            planes
                .par_iter()
                .flat_map(move |rhs_id| {
                    let rhs_verts = &face_vertices[&rhs_id];
                    let rhs_plane = &face_planes[&rhs_id];

                    // Skip comparing with self
                    if lhs_id == rhs_id {
                        return None;
                    }

                    // Skip faces that don't lie on the same plane
                    if !lhs_plane.opposes(rhs_plane) {
                        return None;
                    }

                    // Skip comparing with faces of different vertex count
                    if lhs_verts.len() != rhs_verts.len() {
                        return None;
                    }

                    let vert_count = lhs_verts.len();

                    // Compare vertices
                    let identical_count: usize = lhs_verts
                        .par_iter()
                        .flat_map(|lhs_vert| {
                            rhs_verts.par_iter().map(move |rhs_vert| {
                                let delta = (lhs_vert - rhs_vert).magnitude();
                                if delta < EPSILON {
                                    1
                                } else {
                                    0
                                }
                            })
                        })
                        .sum();

                    if identical_count != vert_count {
                        return None;
                    }

                    // Add faces to set
                    Some([(*lhs_id, *rhs_id), (*rhs_id, *lhs_id)])
                })
                .flatten()
        })
        .collect()
}
