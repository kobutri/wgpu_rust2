use std::collections::VecDeque;
use bytemuck::{Zeroable, Pod};
use rand::Rng;

pub struct Octree {
    pub(crate) data: Vec<Node>,
    pub(crate) depth: i32,
    pub(crate) size: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct Node {
    pub(crate) material_id: i32,
    pub(crate) level: i32,
    pub(crate) sub_voxels: [i32; 8],
    pub(crate) ropes: [i32; 6],
}

impl Octree {
    pub fn new_random(depth: i32, size: f32, chance: f64) -> Self {
        let mut data = vec![];
        Self::new_random_internal(depth, &mut data, chance);
        Self { data, depth, size }
    }

    fn new_random_internal(depth: i32, data: &mut Vec<Node>, chance: f64) {
        data.push(Node {
            material_id: if depth == 0 && rand::thread_rng().gen_bool(chance) {
                Self::SOLID
            } else {
                Self::EMPTY
            },
            ..Default::default()
        });
        let first_address = data.len();
        if data[first_address - 1].material_id & Self::SOLID == 0 && depth != 0 {
            for i in 0..8 {
                let new_address = data.len();
                data[first_address - 1].sub_voxels[i] = new_address as i32;
                Self::new_random_internal(depth - 1, data, chance);
            }
        }
    }

    pub fn new_wall(depth: i32, size: f32) -> Self {
        let mut data = vec![];
        Self::new_wall_internal(depth, &mut data);
        Self { data, depth, size }
    }


    pub fn new_wall_internal(depth: i32, data: &mut Vec<Node>) {
        data.push(Node {
            material_id: if depth == 0 {
                Self::SOLID
            } else {
                Self::EMPTY
            },
            ..Default::default()
        });
        let first_address = data.len();
        if data[first_address - 1].material_id & Self::SOLID == 0 && depth != 0 {
            for i in 0..8 {
                if i & 1 == 0 {
                    let new_adress = data.len();
                    data[first_address - 1].sub_voxels[i] = new_adress as i32;
                    Self::new_wall_internal(depth - 1, data);
                }
            }
        }
    }

    pub fn generate_ropes(data: &mut Vec<Node>) {
        Self::generate_ropes_internal(0, data, &mut VecDeque::from([(0, 0)]));
    }

    fn generate_ropes_internal(root: i32, data: &mut Vec<Node>, stack: &mut VecDeque<(i32, i32)>) {
        for i in 0..6 {
            let voxel_in_dir = Self::generate_rope(stack, data, i);
            data[root as usize].ropes[i as usize] = voxel_in_dir;
        }
        for (index, sub_voxel) in data[root as usize].sub_voxels.clone().iter().enumerate() {
            if *sub_voxel != 0 {
                stack.push_back((*sub_voxel, index as i32));
                Self::generate_ropes_internal(*sub_voxel, data, stack);
                stack.pop_back();
            }
        }
    }

    fn generate_rope(path: &VecDeque<(i32, i32)>, data: &[Node], dir: i32) -> i32 {
        let total_path_len = path.len();
        for i in 1..total_path_len {
            let subvoxel_index = path[total_path_len - i].1;
            let voxel_index = path[total_path_len - i-1].0;
            let subvoxel_dir = match dir {
                0 => 1,
                1 => 1,
                2 => 2,
                3 => 2,
                4 => 4,
                5 => 4,
                _ => unreachable!()
            };
            let positive_dir = dir % 2 == 1;
            if (subvoxel_dir & subvoxel_index != 0 && !positive_dir) || (subvoxel_dir & subvoxel_index == 0 && positive_dir) {
                let mut current_voxel_index = voxel_index;
                for j in total_path_len - i-1..total_path_len - 1 {
                    current_voxel_index = data[current_voxel_index as usize].sub_voxels[(path[j + 1].1 ^ subvoxel_dir) as usize]
                }
                return current_voxel_index;
            }
        }
        -1
    }

    const SOLID: i32 = 1;
    const EMPTY: i32 = 0;
}