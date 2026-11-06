use crate::shaders::{fbm, hash, noise, smoothstep};
use nalgebra_glm::Vec3;

fn pack(c: Vec3) -> u32 {
    ((c.x.clamp(0.0, 255.0) as u32) << 16)
        | ((c.y.clamp(0.0, 255.0) as u32) << 8)
        | c.z.clamp(0.0, 255.0) as u32
}

fn unpack(c: u32) -> Vec3 {
    Vec3::new(
        ((c >> 16) & 255) as f32,
        ((c >> 8) & 255) as f32,
        (c & 255) as f32,
    )
}

/// Fondo procedural fijo en pantalla. Se calcula una sola vez, sin leer imágenes.
pub fn generate(width: usize, height: usize) -> Vec<u32> {
    let mut pixels = vec![0; width * height];
    let extent = height.max(1) as f32;
    for y in 0..height {
        for x in 0..width {
            let p = Vec3::new(
                (x as f32 - width as f32 * 0.5) / extent,
                (y as f32 - height as f32 * 0.5) / extent,
                0.73,
            );
            // La franja galáctica atraviesa la imagen de abajo-izquierda a arriba-derecha.
            let warp = noise(p * 3.2 + Vec3::new(7.0, 2.0, 5.0)) - 0.5;
            let distance = p.y + p.x * 0.55 - 0.04 + warp * 0.18;
            let band = (-distance * distance * 19.0).exp();
            let clouds = fbm(p * 6.0 + Vec3::new(15.0, 3.0, -8.0));
            let wisps = fbm(p * 19.0 + Vec3::new(4.0, 23.0, 6.0));
            let density = smoothstep(0.28, 0.76, clouds) * band;
            let hue = smoothstep(-0.55, 0.6, p.x + (clouds - 0.5) * 1.6);
            let violet = Vec3::new(71.0, 30.0, 108.0);
            let cyan = Vec3::new(20.0, 79.0, 103.0);
            let nebula = violet + (cyan - violet) * hue;
            // Polvo oscuro dentro de las nubes rompe la uniformidad del resplandor.
            let dust = smoothstep(0.44, 0.67, wisps) * band;
            let mut color = Vec3::new(2.0, 3.0, 9.0) + nebula * density * (0.4 + wisps * 0.9);
            color += Vec3::new(23.0, 27.0, 43.0) * band * clouds * 0.3;
            color *= 1.0 - dust * 0.72;
            let vignette = (1.0 - (p.x * p.x + p.y * p.y) * 0.22).clamp(0.55, 1.0);
            pixels[y * width + x] = pack(color * vignette);
        }
    }
    // Semilla fija: cada estrella conserva su posición al cambiar de cuerpo o tiempo.
    let count = width * height / 700;
    for i in 0..count {
        let seed = i as i32;
        let x = hash(seed, 17, 43) * width.saturating_sub(1) as f32;
        let y = hash(seed, 59, 71) * height.saturating_sub(1) as f32;
        let class = hash(seed, 83, 97);
        let bright = class > 0.986;
        let radius = if bright {
            1.45 + hash(seed, 11, 13) * 0.65
        } else {
            0.48 + hash(seed, 11, 13) * 0.5
        };
        let temperature = hash(seed, 101, 103);
        let color = if temperature < 0.25 {
            Vec3::new(159.0, 193.0, 255.0)
        } else if temperature > 0.86 {
            Vec3::new(255.0, 209.0, 158.0)
        } else {
            Vec3::new(229.0, 234.0, 255.0)
        };
        let energy = if bright {
            1.2
        } else {
            0.3 + hash(seed, 107, 109) * 0.8
        };
        let reach = if bright { 11 } else { 3 };
        for py in (y as i32 - reach).max(0)..=(y as i32 + reach).min(height as i32 - 1) {
            for px in (x as i32 - reach).max(0)..=(x as i32 + reach).min(width as i32 - 1) {
                let dx = px as f32 + 0.5 - x;
                let dy = py as f32 + 0.5 - y;
                let d2 = dx * dx + dy * dy;
                // Núcleo suave subpíxel; halo y destellos solo en las estrellas más brillantes.
                let mut intensity = (-d2 / (radius * radius * 0.5)).exp();
                if bright {
                    intensity += 0.14 * (-d2 / (radius * radius * 8.0)).exp();
                    intensity += 0.20
                        * ((-dx.abs() * 0.45 - dy.abs() * 5.0).exp()
                            + (-dy.abs() * 0.45 - dx.abs() * 5.0).exp());
                }
                let index = py as usize * width + px as usize;
                pixels[index] = pack(unpack(pixels[index]) + color * intensity * energy);
            }
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn cosmos_is_repeatable_and_contains_dark_regions_clouds_and_sparse_bright_stars() {
        let pixels = generate(320, 240);
        assert_eq!(pixels.len(), 320 * 240);
        assert_eq!(pixels, generate(320, 240));
        assert!(pixels.iter().copied().collect::<HashSet<_>>().len() > 200);
        let brightness = |p: u32| ((p >> 16) & 255).max((p >> 8) & 255).max(p & 255);
        let dark = pixels.iter().filter(|p| brightness(**p) < 80).count();
        let bright = pixels.iter().filter(|p| brightness(**p) > 180).count();
        assert!(dark > pixels.len() * 3 / 4);
        assert!(
            bright > 10 && bright < pixels.len() / 50,
            "{bright} estrellas brillantes"
        );
    }

    #[test]
    fn small_and_non_square_frames_have_valid_pixel_counts() {
        for (width, height) in [(1, 1), (7, 19), (200, 80)] {
            assert_eq!(generate(width, height).len(), width * height);
        }
    }
}

/// Fondos panorámicos precalculados; cada fotograma solo interpola y desplaza.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Background { Classic, Nebula, Galaxies }
impl Background {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "clasico" | "classic" => Some(Self::Classic),
            "nebulosas" | "nebula" => Some(Self::Nebula),
            "galaxias" | "galaxies" => Some(Self::Galaxies),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self { Self::Classic => "CLASICO", Self::Nebula => "NEBULOSAS", Self::Galaxies => "GALAXIAS" }
    }
    pub fn next(self) -> Self {
        match self { Self::Classic => Self::Nebula, Self::Nebula => Self::Galaxies, Self::Galaxies => Self::Classic }
    }
}
const MAP_W: usize = 640;
const MAP_H: usize = 320;
fn panorama(mode: Background) -> Vec<u32> {
    let mut pixels = vec![0; MAP_W * MAP_H];
    for y in 0..MAP_H {
        let v = y as f32 / (MAP_H - 1) as f32;
        for x in 0..MAP_W {
            let u = x as f32 / MAP_W as f32;
            let angle = u * std::f32::consts::TAU;
            // Un dominio cilíndrico cierra el panorama sin costura horizontal.
            let p = Vec3::new(angle.cos()*2.0, v*5.0, angle.sin()*2.0);
            let density = fbm(p*1.5 + Vec3::new(7.0, 4.0, 19.0));
            let detail = fbm(p*4.5 + Vec3::new(31.0, 3.0, 5.0));
            let clouds = smoothstep(0.30, 0.73, density);
            let hue = noise(p*0.7 + Vec3::repeat(9.0));
            let purple = Vec3::new(98.0, 28.0, 142.0);
            let blue = Vec3::new(16.0, 108.0, 146.0);
            let mut c = Vec3::new(2.0, 4.0, 12.0) + (purple+(blue-purple)*hue)*clouds*(0.24+detail*0.7);
            c *= 1.0-smoothstep(0.51, 0.76, detail)*0.7;
            if mode == Background::Galaxies {
                c *= 0.45;
                // Tres galaxias espirales inclinadas, a diferentes profundidades.
                for (cx, cy, size, tilt) in [(0.15,0.30,0.15,0.45_f32),(0.54,0.70,0.11,-0.65),(0.85,0.43,0.18,0.22)] {
                    let dx = (u-cx+0.5).rem_euclid(1.0)-0.5;
                    let dy = (v-cy)*0.5;
                    let (s, co) = tilt.sin_cos();
                    let gx = (dx*co+dy*s)/size;
                    let gy = (-dx*s+dy*co)/(size*0.43);
                    let r = (gx*gx+gy*gy).sqrt();
                    let arms = (gy.atan2(gx)*2.0-r*8.0+detail*2.5).cos()*0.5+0.5;
                    let disk = (-r*r*2.4).exp();
                    let core = (-r*r*44.0).exp();
                    c += Vec3::new(75.0,100.0,173.0)*disk*arms.powi(3)*1.2;
                    c += Vec3::new(255.0,208.0,148.0)*core*0.95;
                    c += Vec3::new(58.0,32.0,88.0)*disk*0.28;
                }
            }
            pixels[y*MAP_W+x] = pack(c);
        }
    }
    pixels
}

#[derive(Default)]
pub struct Backgrounds {
    nebula: Option<Vec<u32>>,
    galaxies: Option<Vec<u32>>,
    classic: Vec<(usize, usize, Vec<u32>)>,
    last_frame: Option<(usize, usize, Background, f32, Vec<u32>)>,
}
impl Backgrounds {
    pub fn render(&mut self, width: usize, height: usize, mode: Background, time: f32) -> Vec<u32> {
        if let Some((w,h,m,t,pixels))=&self.last_frame {
            if *w==width && *h==height && *m==mode && *t==time {return pixels.clone();}
        }
        let pixels=self.render_uncached(width,height,mode,time);
        self.last_frame=Some((width,height,mode,time,pixels.clone()));
        pixels
    }
    fn render_uncached(&mut self, width: usize, height: usize, mode: Background, time: f32) -> Vec<u32> {
        if width==0 || height==0 {return Vec::new();}
        if mode == Background::Classic {
            if let Some((_,_,pixels)) = self.classic.iter().find(|(w,h,_)| *w==width && *h==height) {
                return pixels.clone();
            }
            let pixels=generate(width,height);
            self.classic.push((width,height,pixels.clone()));
            return pixels;
        }
        let cache = if mode==Background::Nebula { &mut self.nebula } else { &mut self.galaxies };
        let texture = cache.get_or_insert_with(|| panorama(mode));
        let mut pixels=vec![0; width*height];
        let aspect=width as f32/height.max(1) as f32;
        for y in 0..height {
            let v=y as f32/height.max(1) as f32;
            for x in 0..width {
                let u=x as f32/width.max(1) as f32;
                let flow=(time*0.12+v*5.0).sin()*0.004;
                let tx=((u*aspect*0.32+time*0.012+flow).rem_euclid(1.0))*MAP_W as f32;
                let ty=((0.12+v*0.76+(time*0.09+u*4.0).sin()*0.012).clamp(0.0,1.0))*(MAP_H-1) as f32;
                let x0=tx.floor() as usize%MAP_W;let x1=(x0+1)%MAP_W;
                let y0=ty.floor() as usize;let y1=(y0+1).min(MAP_H-1);
                let fx=tx.fract();let fy=ty.fract();
                let top=unpack(texture[y0*MAP_W+x0])*(1.0-fx)+unpack(texture[y0*MAP_W+x1])*fx;
                let bottom=unpack(texture[y1*MAP_W+x0])*(1.0-fx)+unpack(texture[y1*MAP_W+x1])*fx;
                pixels[y*width+x]=pack(top*(1.0-fy)+bottom*fy);
            }
        }
        // Campo de estrellas en tres velocidades: parallax independiente de las nubes.
        for i in 0..(width*height/850).max(1) {
            let seed=i as i32;
            let depth=0.4+hash(seed,32,7)*1.6;
            let x=(hash(seed,17,43)*width as f32-time*3.0*depth).rem_euclid(width.max(1) as f32);
            let y=hash(seed,59,71)*height.saturating_sub(1) as f32;
            let light=0.35+hash(seed,83,97)*0.65;
            let tint=Vec3::new(180.0,211.0,255.0)*light;
            // Deposición subpíxel evita saltos de estrellas entre fotogramas.
            let x0=x.floor() as usize;let y0=y as usize;
            for dx in 0..2 {
                let px=(x0+dx)%width.max(1);
                let weight=if dx==0 {1.0-x.fract()} else {x.fract()};
                let index=y0*width+px;
                pixels[index]=pack(unpack(pixels[index])+tint*weight);
            }
        }
        pixels
    }
}

#[cfg(test)]
mod motion_tests {
    use super::*;
    #[test]
    fn backgrounds_are_reproducible_and_animated_modes_move() {
        let mut backgrounds=Backgrounds::default();
        for mode in [Background::Classic,Background::Nebula,Background::Galaxies] {
            let a=backgrounds.render(160,120,mode,0.0);
            let b=backgrounds.render(160,120,mode,3.0);
            assert_eq!(a,backgrounds.render(160,120,mode,0.0));
            if mode==Background::Classic {assert_eq!(a,b);} else {assert_ne!(a,b);}
            assert_eq!(a.len(),160*120);
        }
        assert_ne!(backgrounds.render(160,120,Background::Nebula,0.0),backgrounds.render(160,120,Background::Galaxies,0.0));
    }
}
