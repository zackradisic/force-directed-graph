use std::collections::BTreeMap;

use crate::{edge::Edge, node::Node};

pub const DEFAULT_STRENGTH: f32 = -100.0;
pub const DEFAULT_MAX_DIST: f32 = 500.0;
pub const DEFAULT_MIN_DIST: f32 = 200.0;

pub struct Physics {
    pub objs: Vec<Object>,
    pub alpha: f32,
    pub alpha_decay: f32,
    pub alpha_target: f32,
}

pub struct Object {
    pub i: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub strength: f32,
}

impl Physics {
    const ALPHA_MIN: f32 = 0.001;
    pub fn new(nodes: &[Node]) -> Self {
        Self {
            objs: nodes
                .into_iter()
                .enumerate()
                .map(|(i, node)| Object::from_node(i as u32, node, DEFAULT_STRENGTH))
                .collect(),
            alpha: 1.0,
            alpha_decay: (1.0 - Self::ALPHA_MIN.powf(1.0 / 900.0)) / 100.0,
            alpha_target: 0.0,
        }
    }

    pub fn tick(
        &mut self,
        dragging: Option<u32>,
        edges: &[Edge],
        edge_map: &BTreeMap<u32, Vec<u32>>,
    ) {
        // self.alpha += (self.alpha_target - self.alpha) * self.alpha_decay;
        // println!("ALPHA: {:?}", self.alpha);
        self.alpha = 1.0;

        let dragging = dragging.map(|x| x as usize).unwrap_or(usize::MAX);
        let len = self.objs.len();
        for i in 0..len {
            if i == dragging {
                continue;
            }
            for j in 0..len {
                let obj = unsafe { self.objs.get_unchecked(i) };
                let other = unsafe { self.objs.get_unchecked(j) };
                let idx = obj.i;
                let x = obj.x;
                let y = obj.y;
                let z = obj.z;
                if idx == other.i {
                    continue;
                }

                let dx = x - other.x;
                let dy = y - other.y;
                let dz = z - other.z;
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                if dist >= DEFAULT_MAX_DIST || dist.is_nan() {
                    continue;
                }
                let force = other.strength * (self.alpha / dist);
                let force_x = force * dx * (self.alpha / dist);
                let force_y = force * dy * (self.alpha / dist);
                let force_z = force * dz * (self.alpha / dist);

                let obj = unsafe { self.objs.get_unchecked_mut(i) };

                obj.x -= force_x;
                obj.y -= force_y;
                obj.z -= force_z;
            }
        }

        for (&node, connections) in edge_map.iter() {
            if node == dragging as u32 {
                continue;
            }

            for other_id in connections {
                let edge = &edges[*other_id as usize];
                let a = &self.objs[node as usize];
                let b = if node == edge.a_id {
                    &self.objs[edge.b_id as usize]
                } else {
                    &self.objs[edge.a_id as usize]
                };

                // let mut dx = a.x - b.x;
                // let mut dy = a.y - b.y;
                // let mut dz = a.z - b.z;
                // let mut l = (dx * dx + dy * dy + dz * dz).sqrt();
                // l = (l - 30.0) / l * -a.strength;
                // dx *= l;
                // dy *= l;
                // dz *= l;
                // let a = &mut self.objs[node as usize];

                // a.x -= dx;
                // a.y -= dy;
                // a.z -= dz;

                let dx = a.x - b.x;
                let dy = a.y - b.y;
                let dz = a.z - b.z;
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                if dist.is_nan() {
                    continue;
                }
                if dist <= DEFAULT_MIN_DIST {
                    continue;
                }

                // let force_x = -a.strength * (dx / dist) * dx;
                // let force_y = -a.strength * (dy / dist) * dy;
                // let force_z = -a.strength * (dz / dist) * dz;
                // let force = a.strength;
                // let force_x = force * (dx / dist).log10();
                // let force_y = force * (dy / dist).log10();
                // let force_z = force * (dz / dist).log10();

                // dist *= 0.0001;
                // let force = -a.strength * dist;
                // let force_x = force * dx;
                // let force_y = force * dy;
                // let force_z = force * dz;
                // println!(
                //     "WTF: {} {} {} dx={} dy={} dz={} force={} dist={}",
                //     force_x, force_y, force_z, dx, dy, dz, force, dist
                // );

                // let force = dist / (a.strength);
                // let force_x = dist / (force * dx);
                // let force_y = dist / (force * dy);
                // let force_z = dist / (force * dz);

                // let force = -a.strength * (self.alpha / dist);
                // let force_x = force * dx * (self.alpha / dist);
                // let force_y = force * dy * (self.alpha / dist);
                // let force_z = force * dz * (self.alpha / dist);

                let dist = dist * 0.00001;
                let force = -a.strength * dist;
                let force_x = (force * dx) * dist;
                let force_y = (force * dy) * dist;
                let force_z = (force * dz) * dist;

                let a = &mut self.objs[node as usize];

                a.x -= force_x;
                a.y -= force_y;
                a.z -= force_z;
            }
        }
    }

    // pub fn tick(&mut self, edges: &[Edge], edge_map: &BTreeMap<u32, Vec<u32>>) {
    //     let len = self.objs.len();
    //     for i in 0..len {
    //         let obj = unsafe { self.objs.get_unchecked(i) };
    //     }
    // }

    pub fn apply(
        &self,
        nodes: &mut [Node],
        edges: &mut [Edge],
        edge_map: &BTreeMap<u32, Vec<u32>>,
    ) {
        assert_eq!(nodes.len(), self.objs.len());

        for (i, obj) in self.objs.iter().enumerate() {
            let node = unsafe { nodes.get_unchecked_mut(i) };
            obj.apply(node);

            edge_map.get(&(i as u32)).map(|node_edges| {
                node_edges.iter().for_each(|edge_id| {
                    let edge = unsafe { edges.get_unchecked_mut(*edge_id as usize) };
                    obj.apply_edge(i as u32, node, edge);
                })
            });
        }
    }
}

impl Object {
    pub fn from_node(i: u32, node: &Node, strength: f32) -> Self {
        Self {
            x: node.position.x,
            y: node.position.y,
            z: node.position.z,
            strength,
            i,
        }
    }

    pub fn apply(&self, node: &mut Node) {
        node.position.x = self.x;
        node.position.y = self.y;
        node.position.z = self.z;
    }

    pub fn apply_edge(&self, id: u32, node: &Node, edge: &mut Edge) {
        if edge.a_id == id {
            edge.a_center = node.position.clone();
        } else if edge.b_id == id {
            edge.b_center = node.position.clone();
        }
    }
}
