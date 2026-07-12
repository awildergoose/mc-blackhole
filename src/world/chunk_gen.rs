use crate::world::{chunk::Chunk, palette::PaletteBlockKind};
use cgmath::Vector3;
use noise::{NoiseFn, Perlin, Seedable};
use rand::{Rng, RngExt, rngs::StdRng};

pub struct ChunkGenerationParams<'a> {
    pub cx: i32,
    pub cz: i32,
    pub chunk: &'a mut Chunk,
    pub random: &'a mut StdRng,
    pub noise: &'a mut Perlin,
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

#[optimize(speed)]
#[expect(clippy::too_many_lines)]
pub fn do_chunk_generation(params: &mut ChunkGenerationParams) {
    // TODO: Multi-thread the first 2 steps :)
    let noise = &mut *params.noise;
    let random = &mut *params.random;

    let decoration_noise = Perlin::new(noise.seed() + 1000);
    let second_decoration_noise = Perlin::new(noise.seed() + 1500);

    let world_scale = 0.005;
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

            let decoration = fbm2d(&decoration_noise, px, pz, 1);
            let second_decoration = fbm2d(&second_decoration_noise, px, pz, 1);

            for ly in -64..y {
                let kind = if ly >= -60 {
                    if ly >= 15 {
                        if decoration >= 0.65 {
                            if decoration >= 0.75 {
                                PaletteBlockKind::Andesite
                            } else {
                                PaletteBlockKind::Diorite
                            }
                        } else {
                            if ly >= 35 {
                                PaletteBlockKind::Granite
                            } else if second_decoration >= 0.4 {
                                PaletteBlockKind::Cobblestone
                            } else {
                                PaletteBlockKind::Stone
                            }
                        }
                    } else {
                        PaletteBlockKind::Deepslate
                    }
                } else {
                    PaletteBlockKind::Bedrock
                };

                params.chunk.set_block_local(lx as u32, ly, lz as u32, kind);
            }
        }
    }

    // Second: Caves
    let cave_noise = Perlin::new(noise.seed() + 2000);
    let cave_scale = 0.04;
    let cave_amp_y = 10.0;
    let cave_octaves = 2;
    let cave_carve_radius = 3;

    let mut cave = None;
    let mut is_single_cave = false;

    for lx in 0..16 {
        for lz in 0..16 {
            let wx = f64::from(lx + params.cx * 16);
            let wz = f64::from(lz + params.cz * 16);

            let px = wx * cave_scale;
            let pz = wz * cave_scale;

            let n = fbm2d(&cave_noise, px, pz, cave_octaves);
            let v = (n * cave_amp_y) - 48.0 - 5.0; // adjust this for more/less caves

            if (-48.0..=cave_amp_y).contains(&v) {
                let h = v + 41.0;
                let y = h.round_ties_even() as i32;

                carve_sphere(lx, y, lz, cave_carve_radius, |x, y, z| {
                    if !(0..16).contains(&x) || !(-64..300).contains(&y) || !(0..16).contains(&z) {
                        return;
                    }
                    params
                        .chunk
                        .set_block_local(x as u32, y, z as u32, PaletteBlockKind::Air);
                });

                // check bounds so we don't get cut off
                if cave.is_none() && lx >= 6 && lz >= 6 && lx <= 10 && lz <= 10 {
                    cave = Some((lx as u32, y, lz as u32));
                    is_single_cave = true;
                } else {
                    is_single_cave = false;
                }
            }
        }
    }

    // Third: Digger Structures
    let max_structures = 3;
    let structure_count = random.next_u32() % max_structures;

    for _ in 0..structure_count {
        // I call this one, digger bot! (or dirt bot)
        let x = random.next_u32() % 16;
        let z = random.next_u32() % 16;

        let top_surface_y = heights[x as usize][z as usize];
        let mut y = top_surface_y + 1;

        // make sure we're above the build limit,
        while y > -64 {
            // if the current block isn't air, then go up and break
            // else, continue moving downwards
            if params.chunk.get_block_local(x, y, z) != PaletteBlockKind::Air {
                y += 1;
                break;
            }

            y -= 1;
        }

        let y = y - 1;
        let mut pos = Vector3::new(x as i32, y, z as i32);

        loop {
            if random.random_bool(0.3) {
                break;
            }

            params
                .chunk
                .set_block_local(pos.x as u32, pos.y, pos.z as u32, PaletteBlockKind::Dirt);

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

            pos.x = pos.x.clamp(0, 15);
            pos.y = pos.y.clamp(0, 300);
            pos.z = pos.z.clamp(0, 15);
        }
    }

    // Fourth: 'Dungeons', Maybe, idk
    if is_single_cave && let Some((lx, y, lz)) = cave {
        // double-check this is a single cave by checking surrounding perlin
        // this is really bad performance and code quality wise
        let chk = |cx, cz| {
            for lx in 0..16 {
                for lz in 0..16 {
                    let wx = f64::from(lx + cx * 16);
                    let wz = f64::from(lz + cz * 16);

                    let px = wx * cave_scale;
                    let pz = wz * cave_scale;

                    let n = fbm2d(&cave_noise, px, pz, cave_octaves);
                    let v = (n * cave_amp_y) - 48.0 - 5.0; // adjust this for more/less caves

                    if (-48.0..=cave_amp_y).contains(&v) {
                        return true;
                    }
                }
            }

            false
        };

        if ![
            chk(params.cx + 1, params.cz),
            chk(params.cx - 1, params.cz),
            chk(params.cx, params.cz + 1),
            chk(params.cx, params.cz - 1),
        ]
        .iter()
        .any(|c| *c)
        {
            println!(
                "ALL GOOD {} {y} {}",
                params.cx * 16 + lx as i32,
                params.cz * 16 + lz as i32
            );

            // walls!
            for lx in lx - cave_carve_radius as u32..lx + cave_carve_radius as u32 {
                params
                    .chunk
                    .set_block_local(lx, y, lz, PaletteBlockKind::CobblestoneWallXF);
            }
            for ly in y - cave_carve_radius..y + cave_carve_radius {
                params
                    .chunk
                    .set_block_local(lx, ly, lz, PaletteBlockKind::CobblestoneWall);
            }
            for lz in lz - cave_carve_radius as u32..lz + cave_carve_radius as u32 {
                params
                    .chunk
                    .set_block_local(lx, y, lz, PaletteBlockKind::CobblestoneWallZF);
            }

            // cobblestone!
            params.chunk.set_block_local(
                lx - cave_carve_radius as u32,
                y,
                lz,
                PaletteBlockKind::Cobblestone,
            );
            params.chunk.set_block_local(
                lx + cave_carve_radius as u32,
                y,
                lz,
                PaletteBlockKind::Cobblestone,
            );
            params.chunk.set_block_local(
                lx,
                y - cave_carve_radius,
                lz,
                PaletteBlockKind::Cobblestone,
            );
            params.chunk.set_block_local(
                lx,
                y + cave_carve_radius,
                lz,
                PaletteBlockKind::Cobblestone,
            );
            params.chunk.set_block_local(
                lx,
                y,
                lz - cave_carve_radius as u32,
                PaletteBlockKind::Cobblestone,
            );
            params.chunk.set_block_local(
                lx,
                y,
                lz + cave_carve_radius as u32,
                PaletteBlockKind::Cobblestone,
            );

            // gold!
            params
                .chunk
                .set_block_local(lx, y, lz, PaletteBlockKind::GoldBlock);
        }
    }
}
