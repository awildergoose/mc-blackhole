use crate::world::palette::PaletteBlockKind;
use cgmath::{InnerSpace, Vector3};
use noise::{NoiseFn, Perlin, Seedable};
use rand::rngs::StdRng;

pub struct ChunkGenerationParams<'lvl> {
    pub cx: i32,
    pub cz: i32,
    pub random: &'lvl mut StdRng,
    pub noise: &'lvl mut Perlin,
}

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

fn flow_direction(noise: &Perlin, pos: Vector3<f32>) -> Vector3<f32> {
    let s = 0.015;
    let yaw = noise.get([
        f64::from(pos.x) * s,
        f64::from(pos.y) * s,
        f64::from(pos.z) * s,
    ]) * std::f64::consts::PI;

    let pitch = noise.get([
        f64::from(pos.x).mul_add(s, 500.0),
        f64::from(pos.y).mul_add(s, 500.0),
        f64::from(pos.z).mul_add(s, 500.0),
    ]) * 0.5;

    Vector3::new(
        (yaw.cos() * pitch.cos()) as f32,
        pitch.sin() as f32,
        (yaw.sin() * pitch.cos()) as f32,
    )
    .normalize()
}

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

#[allow(clippy::similar_names)]
const fn inside_chunk(wx: i32, wz: i32, cx: i32, cz: i32) -> bool {
    let minx = cx * 16;
    let minz = cz * 16;

    wx >= minx && wx < minx + 16 && wz >= minz && wz < minz + 16
}

const fn local(wx: i32, wz: i32, cx: i32, cz: i32) -> (i32, i32) {
    (wx - cx * 16, wz - cz * 16)
}

fn generate_worm<F>(
    noise: &Perlin,
    mut pos: Vector3<f32>,
    length: usize,
    mut radius: f32,
    chunk_x: i32,
    chunk_z: i32,
    mut place_tile: F,
) where
    F: FnMut(i32, i32, i32, PaletteBlockKind),
{
    for step in 0..length {
        let wx = pos.x.round() as i32;
        let wy = pos.y.round() as i32;
        let wz = pos.z.round() as i32;

        // carve the tunnel
        carve_sphere(wx, wy, wz, radius.round() as i32, |x, y, z| {
            if inside_chunk(x, z, chunk_x, chunk_z) {
                let (lx, lz) = local(x, z, chunk_x, chunk_z);

                if y >= 0 {
                    place_tile(lx, y, lz, PaletteBlockKind::Air);
                }
            }
        });

        // occasionally make larger rooms
        if step % 40 == 0 && step != 0 {
            carve_sphere(wx, wy, wz, (radius * 2.5) as i32, |x, y, z| {
                if inside_chunk(x, z, chunk_x, chunk_z) {
                    let (lx, lz) = local(x, z, chunk_x, chunk_z);

                    if y >= 0 {
                        place_tile(lx, y, lz, PaletteBlockKind::Air);
                    }
                }
            });
        }

        let mut dir = flow_direction(noise, pos);

        // keep caves from going straight up
        if dir.y > 0.3 {
            dir.y = 0.3;
            dir = dir.normalize();
        }

        pos += dir * 2.0;

        radius = (noise.get([
            f64::from(pos.x) * 0.05,
            f64::from(pos.y) * 0.05,
            f64::from(pos.z) * 0.05,
        ]) as f32)
            .mul_add(0.15, radius);

        radius = radius.clamp(3.0, 8.0);

        // if we're close to the top, go down
        if pos.y > 180.0 {
            pos.y -= 10.0;
        }

        // if we're close to bedrock, go up
        if pos.y < 5.0 {
            pos.y += 10.0;
        }
    }
}

#[allow(clippy::similar_names)]
#[allow(clippy::cast_precision_loss)]
pub fn do_chunk_generation<F: FnMut(i32, i32, i32, PaletteBlockKind)>(
    params: &mut ChunkGenerationParams,
    mut place_tile: F,
) {
    let noise = &mut *params.noise;
    let decoration_noise = Perlin::new(noise.seed() + 1000);

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

            let kind = if y >= 3 {
                if y >= 20 {
                    if decoration_noise.get([px, pz]) >= 0.6 {
                        PaletteBlockKind::Diorite
                    } else {
                        PaletteBlockKind::Stone
                    }
                } else {
                    PaletteBlockKind::Deepslate
                }
            } else {
                PaletteBlockKind::Bedrock
            };

            for ly in -64..y {
                place_tile(lx, ly, lz, kind);
            }
        }
    }

    // Second: Carvers
    let cave_noise = Perlin::new(noise.seed() + 2000);

    let world_cx = params.cx * 16;
    let world_cz = params.cz * 16;

    let cell_size = 64;

    let cell_x = world_cx.div_euclid(cell_size);
    let cell_z = world_cz.div_euclid(cell_size);

    for gx in cell_x - 1..=cell_x + 1 {
        for gz in cell_z - 1..=cell_z + 1 {
            let wx = gx * cell_size + cell_size / 2;
            let wz = gz * cell_size + cell_size / 2;

            let chance = cave_noise.get([f64::from(gx) * 0.5, f64::from(gz) * 0.5]);

            if chance < -0.15 {
                continue;
            }

            let start_x =
                (cave_noise.get([f64::from(gx), f64::from(gz)]) as f32).mul_add(20.0, wx as f32);
            let start_y = ((cave_noise.get([f64::from(gx) * 0.2, f64::from(gz) * 0.2]) + 1.0)
                as f32)
                .mul_add(30.0, 30.0);
            let start_z = (cave_noise.get([f64::from(gx) + 100.0, f64::from(gz) + 100.0]) as f32)
                .mul_add(20.0, wz as f32);

            generate_worm(
                &cave_noise,
                Vector3::new(start_x, start_y, start_z),
                220,
                5.0,
                params.cx,
                params.cz,
                &mut place_tile,
            );
        }
    }
}
