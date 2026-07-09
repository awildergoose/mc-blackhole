use std::{rc::Rc, sync::nonpoison::Mutex};

use crate::world::palette::PaletteBlockKind;
use cgmath::Vector3;
use noise::{NoiseFn, Perlin, Seedable};
use rand::{Rng, RngExt, rngs::StdRng};

pub struct ChunkGenerationParams<'lvl> {
    pub cx: i32,
    pub cz: i32,
    pub random: &'lvl mut StdRng,
    pub noise: &'lvl mut Perlin,
}

#[optimize(speed)]
fn fbm2d(perlin: &Perlin, x: f64, z: f64, octaves: u32) -> f64 {
    let mut amp = 1.0;
    let mut freq = 1.0;
    let mut sum = 0.0;
    let mut norm = 0.0;

    for _ in 0..octaves {
        sum += amp * perlin.get([x * freq, z * freq]);
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }

    sum / norm
}

#[optimize(speed)]
fn carve_sphere<F>(cx: i32, cy: i32, cz: i32, radius: i32, mut place: F)
where
    F: FnMut(i32, i32, i32),
{
    let r2 = radius * radius;

    for x in cx - radius..=cx + radius {
        for y in cy - radius..=cy + radius {
            for z in cz - radius..=cz + radius {
                let dx = x - cx;
                let dy = y - cy;
                let dz = z - cz;

                if dx * dx + dy * dy + dz * dz <= r2 {
                    place(x, y, z);
                }
            }
        }
    }
}

pub enum ChunkRequest {
    SetTile {
        x: i32,
        y: i32,
        z: i32,
        kind: PaletteBlockKind,
    },
    GetTile {
        x: i32,
        y: i32,
        z: i32,
    },
}

pub enum ChunkResponse {
    None,
    GetTile { tile: PaletteBlockKind },
}

#[allow(clippy::similar_names)]
#[allow(clippy::cast_precision_loss)]
#[allow(clippy::too_many_lines)]
#[optimize(speed)]
pub fn do_chunk_generation<F: FnMut(ChunkRequest) -> ChunkResponse>(
    params: &mut ChunkGenerationParams,
    chk_req: Rc<Mutex<F>>,
) {
    // TODO: Multi-thread the first 2 steps :)
    let noise = &mut *params.noise;
    let random = &mut *params.random;

    let decoration_noise = Perlin::new(noise.seed() + 1000);

    let chk_req2 = chk_req.clone();
    let get_tile = move |x, y, z| {
        chk_req2.with_mut(|chk| match chk(ChunkRequest::GetTile { x, y, z }) {
            ChunkResponse::None => unreachable!(),
            ChunkResponse::GetTile { tile } => tile,
        })
    };
    let place_tile = move |x, y, z, kind| {
        chk_req.with_mut(|chk| chk(ChunkRequest::SetTile { x, y, z, kind }));
    };

    let world_scale = 0.0007;
    let base_y = 20.0;
    let amp_y = 35.0;
    let octaves = 5;

    let mut heights = [[0i32; 16]; 16];

    // First: FBM
    for lx in 0..16 {
        for lz in 0..16 {
            let wx = f64::from(lx + params.cx * 16);
            let wz = f64::from(lz + params.cz * 16);

            let px = wx * world_scale;
            let pz = wz * world_scale;

            let n = fbm2d(noise, px, pz, octaves);
            let mut h = base_y + (n * amp_y);
            h = h.clamp(0.0, 300.0);

            let y = h.round_ties_even() as i32;
            heights[lx as usize][lz as usize] = y;

            let decoration = fbm2d(&decoration_noise, px, pz, 2);

            for ly in -64..y {
                let kind = if ly >= -60 {
                    if ly >= 15 {
                        if decoration >= 0.75 {
                            if decoration >= 0.85 {
                                PaletteBlockKind::Andesite
                            } else {
                                PaletteBlockKind::Diorite
                            }
                        } else {
                            PaletteBlockKind::Stone
                        }
                    } else {
                        PaletteBlockKind::Deepslate
                    }
                } else {
                    PaletteBlockKind::Bedrock
                };

                place_tile(lx, ly, lz, kind);
            }
        }
    }

    // Second: Caves
    let cave_noise = Perlin::new(noise.seed() + 2000);
    let cave_scale = 0.05;
    let cave_amp_y = 5000.0;
    let cave_octaves = 1;

    for lx in 0..16 {
        for lz in 0..16 {
            let wx = f64::from(lx + params.cx * 16);
            let wz = f64::from(lz + params.cz * 16);

            let px = wx * cave_scale;
            let pz = wz * cave_scale;

            let n = fbm2d(&cave_noise, px, pz, cave_octaves);
            let v = n * cave_amp_y;

            if v.abs() >= 1500.0 {
                let h = v.clamp(0.0, 300.0);
                let y = h.round_ties_even() as i32;

                carve_sphere(lx, y, lz, 3, |x, y, z| {
                    place_tile(x, y, z, PaletteBlockKind::Air);
                });
            }
        }
    }

    // Third: Structures
    let max_structures = 3;
    let structure_count = random.next_u32() % max_structures;

    for _ in 0..structure_count {
        // I call this one, digger bot! (or dirt bot)
        let x = (random.next_u32() % 16) as i32;
        let z = (random.next_u32() % 16) as i32;

        let top_surface_y = heights[x as usize][z as usize];
        let mut y = top_surface_y + 1;

        // if we're above the build limit, OR
        // if the current block is air, then go down more
        while y > -64 {
            if get_tile(x, y, z) != PaletteBlockKind::Air {
                y += 1;
                break;
            }

            y -= 1;
        }

        let y = y - 1;
        let mut pos = Vector3::new(x, y, z);

        loop {
            if random.random_bool(0.3) {
                break;
            }

            place_tile(pos.x, pos.y, pos.z, PaletteBlockKind::Dirt);

            let directions = [
                Vector3::new(1, 0, 0),
                Vector3::new(0, 1, 0),
                Vector3::new(0, 0, 1),
            ];
            let mut dir = directions[random.next_u32() as usize % directions.len()];
            if random.random_bool(0.5) {
                dir = -dir;
            }

            pos += dir;

            pos.x = pos.x.clamp(0, 16);
            pos.y = pos.y.clamp(-64, 300);
            pos.z = pos.z.clamp(0, 16);
        }
    }
}
