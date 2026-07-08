use noise::{NoiseFn, Perlin};
use rand::rngs::StdRng;

use crate::world::palette::PaletteBlockKind;

pub struct ChunkGenerationParams<'lvl> {
    pub cx: i32,
    pub cz: i32,
    pub random: &'lvl mut StdRng,
    pub noise: &'lvl mut Perlin,
}

fn remap<T>(value: T, istart: T, istop: T, ostart: T, ostop: T) -> T
where
    T: Copy
        + std::ops::Add<Output = T>
        + std::ops::Sub<Output = T>
        + std::ops::Mul<Output = T>
        + std::ops::Div<Output = T>,
{
    ostart + (ostop - ostart) * ((value - istart) / (istop - istart))
}

pub fn do_chunk_generation<F: FnMut(i32, i32, i32, PaletteBlockKind)>(
    params: &mut ChunkGenerationParams,
    mut place_tile: F,
) {
    // let random = &mut *params.random;
    let noise = &mut *params.noise;

    for x in 0..16 {
        for z in 0..16 {
            let y = noise.get([
                (f64::from(x + (params.cx * 16)) / 3000.0),
                (f64::from(z + (params.cz * 16)) / 3000.0),
            ]);
            let y = remap(y, -1.0, 1.0, 0.0, 150.0);
            let y = y.round();
            let y = y as i32;
            place_tile(x, y, z, PaletteBlockKind::Stone);
        }
    }
}
