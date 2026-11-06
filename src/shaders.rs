use crate::color::Color;
use nalgebra_glm::Vec3;
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shader { Mogo, Cybertron, Praxis, Sirius, Sun, Starkiller, Earth, Moon, Rings }
pub fn light_direction() -> Vec3 {
    Vec3::new(-0.6, 0.8, 1.0).normalize()
}

fn rgb(hex: u32) -> Vec3 {
    Vec3::new(
        ((hex >> 16) & 255) as f32,
        ((hex >> 8) & 255) as f32,
        (hex & 255) as f32,
    )
}

fn mix(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

pub(crate) fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub(crate) fn hash(x: i32, y: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374761393)
        ^ (y as u32).wrapping_mul(668265263)
        ^ (z as u32).wrapping_mul(2147483647);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    (h ^ (h >> 16)) as f32 / u32::MAX as f32
}

/// Value noise tridimensional: no hay costura de UV ni imágenes precalculadas.
pub(crate) fn noise(p: Vec3) -> f32 {
    let base = p.map(|v| v.floor());
    let f = (p - base).map(|v| v * v * (3.0 - 2.0 * v));
    let mut result = 0.0;
    for z in 0..2 {
        for y in 0..2 {
            for x in 0..2 {
                let weight = if x == 0 { 1.0 - f.x } else { f.x }
                    * if y == 0 { 1.0 - f.y } else { f.y }
                    * if z == 0 { 1.0 - f.z } else { f.z };
                result += weight * hash(base.x as i32 + x, base.y as i32 + y, base.z as i32 + z);
            }
        }
    }
    result
}

pub(crate) fn fbm(mut p: Vec3) -> f32 {
    let mut value = 0.0;
    let mut weight = 0.5;
    for _ in 0..4 {
        value += noise(p) * weight;
        p = p * 2.03 + Vec3::new(13.1, 7.7, 19.3);
        weight *= 0.5;
    }
    value / 0.9375
}


fn line(d: f32, width: f32) -> f32 { 1.0-smoothstep(width, width*1.6, d.abs()) }
fn uv(p: Vec3) -> (f32,f32) { (p.x.atan2(p.z), p.y.asin()) }
const FACE: &[u8] = include_bytes!("../assets/luis_miguel.rgb");
/// Simplified geographic outlines in degrees. No external textures or downloads.
fn earth_land(lon:f32,lat:f32)->f32 {
    const AFRICA:&[(f32,f32)]=&[(-17.0,36.0),(10.0,37.0),(32.0,31.0),(35.0,15.0),(51.0,12.0),(42.0,-12.0),(32.0,-29.0),(18.0,-35.0),(11.0,-18.0),(9.0,4.0),(-15.0,10.0)];
    const EUROASIA:&[(f32,f32)]=&[(-10.0,36.0),(-9.0,44.0),(3.0,50.0),(7.0,58.0),(25.0,71.0),(60.0,70.0),(100.0,77.0),(145.0,67.0),(179.0,64.0),(160.0,50.0),(140.0,38.0),(120.0,23.0),(109.0,5.0),(99.0,10.0),(79.0,8.0),(68.0,25.0),(52.0,15.0),(39.0,30.0),(28.0,40.0),(13.0,38.0),(0.0,43.0)];
    const NORTH:&[(f32,f32)]=&[(-168.0,71.0),(-137.0,70.0),(-109.0,76.0),(-58.0,53.0),(-82.0,25.0),(-88.0,21.0),(-84.0,10.0),(-105.0,23.0),(-118.0,33.0),(-130.0,54.0),(-168.0,61.0)];
    const SOUTH:&[(f32,f32)]=&[(-81.0,12.0),(-61.0,10.0),(-35.0,-6.0),(-42.0,-23.0),(-54.0,-35.0),(-69.0,-55.0),(-76.0,-40.0),(-80.0,-5.0)];
    const AUSTRALIA:&[(f32,f32)]=&[(113.0,-22.0),(130.0,-12.0),(141.0,-11.0),(153.0,-26.0),(145.0,-39.0),(130.0,-33.0),(115.0,-35.0)];
    const GREENLAND:&[(f32,f32)]=&[(-55.0,60.0),(-22.0,70.0),(-19.0,81.0),(-47.0,84.0),(-65.0,76.0)];
    const MADAGASCAR:&[(f32,f32)]=&[(44.0,-13.0),(51.0,-15.0),(47.0,-26.0),(43.0,-23.0)];
    let x=lon.to_degrees()+15.0;
    let x=if x>180.0 {x-360.0} else {x};let y=lat.to_degrees();
    let mut signed_distance:f32=-180.0;
    for polygon in [AFRICA,EUROASIA,NORTH,SOUTH,AUSTRALIA,GREENLAND,MADAGASCAR] {
        let mut inside=false;let mut distance:f32=360.0;
        for i in 0..polygon.len(){
            let (ax,ay)=polygon[i];let (bx,by)=polygon[(i+1)%polygon.len()];
            if (ay>y)!=(by>y) && x<(bx-ax)*(y-ay)/(by-ay)+ax {inside=!inside;}
            let dx=bx-ax;let dy=by-ay;
            let t=(((x-ax)*dx+(y-ay)*dy)/(dx*dx+dy*dy)).clamp(0.0,1.0);
            distance=distance.min(((x-ax-t*dx).powi(2)+(y-ay-t*dy).powi(2)).sqrt());
        }
        signed_distance=signed_distance.max(if inside {distance} else {-distance});
    }
    if y < -72.0 { return 1.0; }
    smoothstep(-0.6,0.6,signed_distance)
}
impl Shader {
    /// Four cumulative stages: base, structure, identifying detail, emission/atmosphere.
    pub fn shade(self, p: Vec3, normal: Vec3, view: Vec3, time: f32, layers: u8) -> Color {
        let l=layers.clamp(1,4);
        let rim=(1.0-normal.dot(&view).abs()).clamp(0.0,1.0).powi(3);
        let diffuse=normal.dot(&light_direction()).max(0.0);
        let (lon,lat)=uv(p);
        let terrain=fbm(p*4.3+Vec3::repeat(11.0));
        let land=if self==Self::Earth { earth_land(lon,lat) } else {0.0};
        let mut emission=Vec3::zeros();
        let mut c=match self {
            Self::Mogo => mix(rgb(0x171323),rgb(0x76935D),smoothstep(0.40,0.58,terrain)),
            Self::Cybertron => mix(rgb(0x132B41),rgb(0x6C8092),terrain),
            Self::Praxis => mix(rgb(0x15192F),rgb(0x835339),terrain),
            Self::Sirius => mix(rgb(0x177BBE),rgb(0x9DEBFF),terrain),
            Self::Sun => mix(rgb(0xE89412),rgb(0xFFF37E),terrain),
            Self::Starkiller => mix(rgb(0x263143),rgb(0xC4D3E1),smoothstep(0.40,0.58,terrain)),
            Self::Earth => mix(rgb(0x06336E),rgb(0x3F8851),land),
            Self::Moon => mix(rgb(0x454553),rgb(0xC2BEB7),terrain),
            Self::Rings => rgb(0x13FF35),
        };
        if l>=2 {
            match self {
                Self::Cybertron | Self::Starkiller => {
                    let grid=line((lon*18.0).sin(),0.055).max(line((lat*32.0).sin(),0.055));
                    c*=0.65+noise(p*100.0)*0.45;
                    c=mix(c,rgb(0x050F1D),grid*0.92);
                    let plates=noise(Vec3::new((lon*24.0).floor(),(lat*36.0).floor(),2.0));
                    c*=0.65+plates*0.65;
                },
                Self::Sirius | Self::Sun => {
                    let plasma=fbm(p*13.0+Vec3::new(time*0.07,0.0,-time*0.04));
                    c*=0.65+plasma*0.6;
                    c+=rgb(if self==Self::Sun {0xFFD644} else {0x5AD9FF})*line(plasma-0.52,0.022)*0.35;
                },
                Self::Earth => {
                    c=mix(c,rgb(0xB5A270),land*smoothstep(0.50,0.65,terrain)*0.75);
                    c=mix(c,rgb(0xE9F6FF),smoothstep(0.82,0.95,p.y.abs()+(terrain-0.5)*0.2));
                },
                Self::Mogo => c*=0.7+fbm(p*23.0)*0.6,
                Self::Praxis => {
                    let fault=p.x+0.16*(p.y*7.0).sin()+0.12*(terrain-0.5);
                    c=mix(c,rgb(0xE16E25),1.0-smoothstep(0.08,0.27,fault.abs()));
                },
                Self::Moon => c*=0.65+noise(p*65.0)*0.6,
                Self::Rings => c*=0.65+0.3*(p.norm()*130.0).sin().abs(),
            }
        }
        if l>=3 {
            match self {
                Self::Mogo => {
                    // Green Lantern emblem lies on the actual spherical surface.
                    let x=p.x; let y=p.dot(&Vec3::new(0.0,0.934,-0.357));
                    let front=smoothstep(0.35,0.65,p.dot(&Vec3::new(0.0,0.357,0.934)));
                    let outer=line((x*x+y*y).sqrt()-0.63,0.045);
                    let inner=line((x*x+y*y).sqrt()-0.24,0.045);
                    let bars=line(y.abs()-0.32,0.045)*(1.0-smoothstep(0.31,0.35,x.abs()));
                    let mark=outer.max(inner).max(bars)*front;
                    c=mix(c,rgb(0x0CEB29),mark); emission+=rgb(0x11FF23)*mark*0.65;
                },
                Self::Cybertron => {
                    let seams=line((lon*11.0+lat*6.0+terrain*2.0).sin(),0.065);
                    let circuits=line((lat*49.0).sin(),0.04)*smoothstep(0.45,0.7,noise(p*22.0));
                    emission+=rgb(0xFF6A08)*(seams+circuits)*0.95;
                    for center in [Vec3::new(-0.35,0.52,0.78),Vec3::new(0.45,-0.24,0.86)] {
                        let d=(p-center.normalize()).norm();
                        c=mix(c,rgb(0x0C1E30),1.0-smoothstep(0.15,0.19,d));
                        emission+=rgb(0xFF9C19)*line(d-0.23,0.017);
                    }
                },
                Self::Praxis => {
                    let dots=line((lon*40.0).sin(),0.13)*line((lat*32.0).sin(),0.13);
                    emission+=rgb(0x1689FF)*dots*1.4;
                    let fault=p.x+0.16*(p.y*7.0).sin();
                    emission+=rgb(0xFF8B32)*line(fault,0.055)*(0.7+0.3*(time*1.5).sin());
                },
                Self::Starkiller => {
                    let trench=line(lat+0.04,0.035);
                    c=mix(c,rgb(0x090D1B),trench);
                    emission+=rgb(0xFF1533)*trench*0.55;
                    let d=(p-Vec3::new(-0.35,0.34,0.87).normalize()).norm();
                    c=mix(c,rgb(0x300512),1.0-smoothstep(0.18,0.23,d));
                    emission+=rgb(0xFF1533)*line(d-0.22,0.024);
                    emission+=rgb(0xFFBAC0)*(1.0-smoothstep(0.025,0.09,d))*1.2;
                },
                Self::Sun => {
                    // Front-facing elliptical portrait sampled from the supplied reference.
                    let x=p.x/0.70; let y=p.dot(&Vec3::new(0.0,0.934,-0.357))/0.80;
                    let front=smoothstep(0.35,0.65,p.dot(&Vec3::new(0.0,0.357,0.934)));
                    let mask=(1.0-smoothstep(0.78,1.0,x*x+y*y))*front;
                    let ix=(((x+1.0)*0.5*191.0).clamp(0.0,191.0)) as usize;
                    let iy=(((1.0-y)*0.5*199.0).clamp(0.0,199.0)) as usize;
                    let i=(iy*192+ix)*3;
                    let face=Vec3::new(FACE[i] as f32,FACE[i+1] as f32,FACE[i+2] as f32);
                    c=mix(c,face,mask*0.96);
                },
                Self::Sirius => c+=rgb(0xB3F5FF)*smoothstep(0.54,0.78,noise(p*95.0+Vec3::repeat(time*0.1)))*0.5,
                Self::Earth => {
                    let clouds=fbm(p*8.0+Vec3::new(time*0.02,0.0,0.0));
                    c=mix(c,rgb(0xF4FAFF),smoothstep(0.54,0.68,clouds)*0.9);
                },
                Self::Moon => {
                    for i in 0..18 {
                        let center=Vec3::new(hash(i,0,1)*2.0-1.0,hash(i,2,3)*2.0-1.0,hash(i,4,5)*2.0-1.0).normalize();
                        let d=(p-center).norm()/(0.055+hash(i,6,7)*0.12);
                        c*=1.0-(1.0-smoothstep(0.65,0.95,d))*0.55;
                        c+=rgb(0x9C9A91)*line(d-1.0,0.12)*0.3;
                    }
                },
                Self::Rings => {},
            }
        }
        let emissive=matches!(self,Self::Sun|Self::Sirius|Self::Rings);
        let mut color=c*if emissive {0.88+0.12*diffuse} else {0.15+0.85*diffuse}+emission;
        if l>=4 {
            let atmosphere=match self {
                Self::Mogo|Self::Rings=>0x18FF38,Self::Cybertron=>0x2785E8,
                Self::Praxis=>0x3255F0,Self::Sirius=>0x64E5FF,Self::Sun=>0xFFBC21,
                Self::Starkiller=>0xDB224B,Self::Earth=>0x268BFF,Self::Moon=>0x4A5777,
            };
            color+=rgb(atmosphere)*rim*0.65;
        }
        Color::new(color.x.clamp(0.0,255.0) as u8,color.y.clamp(0.0,255.0) as u8,color.z.clamp(0.0,255.0) as u8)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn geographic_outlines_distinguish_africa_from_the_atlantic() {
        // Shader longitude is offset by fifteen degrees.
        assert!(earth_land(5.0_f32.to_radians(),0.0)>0.9);
        assert!(earth_land((-45.0_f32).to_radians(),0.0)<0.1);
    }
    #[test]
    fn stars_are_emissive_on_the_unlit_hemisphere() {
        let n=-light_direction();
        for shader in [Shader::Sun,Shader::Sirius] {
            let c=shader.shade(n,n,n,0.0,4).to_hex();
            assert!(((c>>16)&255)+(c&255)>100);
        }
    }
    #[test]
    fn portrait_asset_is_complete_and_noise_handles_negative_coordinates() {
        assert_eq!(FACE.len(),192*200*3);
        for i in -20..20 {
            let p=Vec3::new(i as f32*0.13,-0.72,1.31);
            let v=fbm(p);assert!((0.0..=1.0).contains(&v));assert_eq!(v,fbm(p));
            assert!((v-fbm(p+Vec3::new(0.0001,0.0,0.0))).abs()<0.001);
        }
    }
}
