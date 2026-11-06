use nalgebra_glm::Vec3;
use std::error::Error;

use crate::shaders::Shader;
use crate::triangle::{shaded_triangle, wireframe};
use crate::{camera::Camera, color::Color, framebuffer::Framebuffer, obj::Obj};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Body { Mogo, Cybertron, Praxis, Sirius, Sun, Starkiller, Earth }
impl Body {
    pub const ALL: [Self;7]=[Self::Mogo,Self::Cybertron,Self::Praxis,Self::Sirius,Self::Sun,Self::Starkiller,Self::Earth];
    pub fn name(self)-> &'static str { match self {
        Self::Mogo=>"MOGO",Self::Cybertron=>"CYBERTRON",Self::Praxis=>"PRAXIS",Self::Sirius=>"SIRIO",Self::Sun=>"SOL / LUIS MIGUEL",Self::Starkiller=>"STARKILLER BASE",Self::Earth=>"TIERRA Y LUNA" } }
    pub fn slug(self)-> &'static str { match self {
        Self::Mogo=>"mogo",Self::Cybertron=>"cybertron",Self::Praxis=>"praxis",Self::Sirius=>"sirio",Self::Sun=>"sol",Self::Starkiller=>"starkiller",Self::Earth=>"tierra" } }
    pub fn parse(s:&str)->Option<Self> { Self::ALL.into_iter().find(|b| b.slug()==s).or_else(|| match s {"sirius"=>Some(Self::Sirius),"sun"=>Some(Self::Sun),"earth"=>Some(Self::Earth),_=>None}) }
    fn radius(self,extras:bool)->f32 { if extras && self==Self::Earth {2.0} else if extras && self==Self::Mogo {2.3} else {1.24} }
}

pub struct Models {
    star: Obj,
    rocky: Obj,
    gas: Obj,
    moon: Obj,
    rings: Obj,
}

impl Models {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let load =
            |name: &str| Obj::load(&format!("{}/models/{name}.obj", env!("CARGO_MANIFEST_DIR")));
        Ok(Self {
            star: load("estrella")?,
            rocky: load("rocoso")?,
            gas: load("gaseoso")?,
            moon: load("luna")?,
            rings: load("anillos")?,
        })
    }
}

pub fn camera() -> Camera {
    Camera::new(Vec3::new(0.0, 2.3, 6.0), Vec3::zeros(), Vec3::y())
}

struct Object<'a> {
    mesh: &'a Obj,
    shader: Shader,
    center: Vec3,
    scale: f32,
    angle: f32,
}

fn rotate(p: Vec3, angle: f32) -> Vec3 {
    let (s, c) = angle.sin_cos();
    Vec3::new(c * p.x + s * p.z, p.y, -s * p.x + c * p.z)
}

/// Una sola escala para toda la escena; luna, anillos y planeta comparten z-buffer.
pub fn render(
    fb: &mut Framebuffer,
    models: &Models,
    body: Body,
    camera: &Camera,
    time: f32,
    extras: bool,
    hologram: bool,
    layers: u8,
) {
    fb.clear();
    let mut objects = vec![Object {
        mesh: match body { Body::Sun|Body::Sirius=>&models.star,Body::Mogo=>&models.gas,_=>&models.rocky },
        shader: match body { Body::Mogo=>Shader::Mogo,Body::Cybertron=>Shader::Cybertron,Body::Praxis=>Shader::Praxis,Body::Sirius=>Shader::Sirius,Body::Sun=>Shader::Sun,Body::Starkiller=>Shader::Starkiller,Body::Earth=>Shader::Earth },
        center: Vec3::zeros(),
        scale: 1.0,
        angle: time * 0.10,
    }];
    if extras && body == Body::Earth {
        let angle = time * 0.22 + 0.3;
        objects.push(Object {
            mesh: &models.moon,
            shader: Shader::Moon,
            center: Vec3::new(1.60 * angle.cos(), 0.32, 1.60 * angle.sin()),
            scale: 0.24,
            angle: time * 0.10,
        });
    }
    if extras && body == Body::Mogo {
        objects.push(Object {
            mesh: &models.rings,
            shader: Shader::Rings,
            center: Vec3::zeros(),
            scale: 1.0,
            angle: 0.0,
        });
    }
    let right = camera.basis_change(&Vec3::x());
    let up = camera.basis_change(&Vec3::y());
    let back = camera.basis_change(&Vec3::z());
    let scale = 0.86 * fb.width.min(fb.height) as f32 / (2.0 * body.radius(extras));
    if layers >= 4 && !hologram && matches!(body, Body::Sun | Body::Sirius) {
        // Corona behind the mesh. It uses the same projection and object scale.
        let tint = if body == Body::Sun { Color::from_hex(0xFFB51C) } else { Color::from_hex(0x39CFFF) };
        let cx=fb.width as f32*0.5;let cy=fb.height as f32*0.5;
        for y in 0..fb.height { for x in 0..fb.width {
            let dx=(x as f32+0.5-cx)/scale;let dy=(y as f32+0.5-cy)/scale;
            let r=(dx*dx+dy*dy).sqrt();
            if (0.97..1.24).contains(&r) {
                let angle=dy.atan2(dx);
                let rays=(angle*21.0+0.5*(angle*7.0+time*0.15).sin()).sin().abs().powi(8);
                let glow=(-((r-1.0).max(0.0))*22.0).exp()*0.48;
                let flame=if body==Body::Sun {rays*(1.0-(r-1.0)/0.23).max(0.0)*0.55} else {rays*glow*0.3};
                let i=y*fb.width+x;
                fb.buffer[i]=(Color::from_hex(fb.buffer[i])+tint*(glow+flame)).to_hex();
            }
        }}
    }
    for object in objects {
        let world: Vec<Vec3> = object
            .mesh
            .vertices
            .iter()
            .map(|p| rotate(*p * object.scale, object.angle) + object.center)
            .collect();
        let screen: Vec<Vec3> = world
            .iter()
            .map(|p| {
                let relative = p - camera.eye;
                Vec3::new(
                    fb.width as f32 * 0.5 + relative.dot(&right) * scale,
                    fb.height as f32 * 0.5 - relative.dot(&up) * scale,
                    -relative.dot(&back),
                )
            })
            .collect();
        for face in &object.mesh.faces {
            for i in 1..face.len() - 1 {
                let indices = [face[0], face[i], face[i + 1]];
                let projected = indices.map(|j| screen[j]);
                if hologram {
                    fb.set_current_color(Color::new(35, 195, 255).to_hex());
                    wireframe(fb, &projected[0], &projected[1], &projected[2]);
                    continue;
                }
                let local = indices.map(|j| object.mesh.vertices[j]);
                let face_normal = (local[1] - local[0]).cross(&(local[2] - local[0]));
                if face_normal.norm_squared() < 1e-12 {
                    continue;
                }
                let face_normal = rotate(face_normal.normalize(), object.angle);
                // Solo los anillos necesitan las dos caras; las esferas son cerradas.
                let view = back;
                if object.shader != Shader::Rings && face_normal.dot(&view) <= 0.0 {
                    continue;
                }
                shaded_triangle(fb, &projected, &local, |p| {
                    let is_ring = object.shader == Shader::Rings;
                    let normal = if is_ring {
                        if face_normal.dot(&view) < 0.0 {
                            -face_normal
                        } else {
                            face_normal
                        }
                    } else {
                        rotate(p.normalize(), object.angle)
                    };
                    // La proyección es ortográfica: todos los rayos de vista son paralelos.
                    let color = object.shader.shade(
                        if is_ring { p } else { p.normalize() },
                        normal,
                        back,
                        time,
                        layers,
                    );
                    color
                });
            }
        }
    }
    if body==Body::Starkiller && extras && layers>=4 && !hologram {
        let direction=rotate(Vec3::new(-0.35,0.34,0.87).normalize(),time*0.10);
        let origin=direction*1.015;
        // Camera-independent beam aligned with the aperture; rear emission is occluded.
        let end=origin+direction*0.85;
        let project=|p:Vec3| {
            let q=p-camera.eye;
            Vec3::new(fb.width as f32*0.5+q.dot(&right)*scale,fb.height as f32*0.5-q.dot(&up)*scale,-q.dot(&back))
        };
        let a=project(origin);let b=project(end);
        let delta=b-a;let steps=(delta.x.abs().max(delta.y.abs()).ceil() as usize).max(1);
        for j in 0..=steps {
            let t=j as f32/steps as f32;let point=a+delta*t;
            let width=(scale*(0.028+0.018*t)) as i32;
            for dy in -width*3..=width*3 {for dx in -width*3..=width*3 {
                let x=point.x as i32+dx;let y=point.y as i32+dy;
                let d=((dx*dx+dy*dy) as f32).sqrt()/width.max(1) as f32;
                if x<0||y<0||!fb.is_visible(x as usize,y as usize,point.z){continue}
                if d<1.0 {
                    fb.set_current_color(Color::from_hex(if d<0.4 {0xFFF1F2} else {0xFF214B}).to_hex());
                    fb.point_with_depth(x as usize,y as usize,point.z);
                } else if d<3.0 {
                    fb.set_current_color(0xFF143D);fb.blend_point(x,y,(3.0-d)*0.035);
                }
            }}
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_seven_bodies_and_four_cumulative_layers_render_distinctly() {
        let models=Models::load().unwrap();
        let mut fb=Framebuffer::new(192,160);
        let mut finals=Vec::new();
        for body in Body::ALL {
            let mut previous=None;
            for layer in 1..=4 {
                render(&mut fb,&models,body,&camera(),0.0,true,false,layer);
                assert!(fb.buffer.iter().filter(|p| **p!=0).count()>400,"{body:?}: empty render");
                if let Some(image)=previous {assert_ne!(fb.buffer,image,"{body:?}, layer {layer}");}
                previous=Some(fb.buffer.clone());
            }
            assert!(!finals.contains(&fb.buffer),"duplicate body {body:?}");
            finals.push(fb.buffer.clone());
        }
    }
    #[test]
    fn moon_orbits_in_front_and_behind_earth_and_render_is_reproducible() {
        let models=Models::load().unwrap();let mut fb=Framebuffer::new(240,200);
        let mut images=Vec::new();
        for time in [0.0,6.0,14.0,22.0] {
            render(&mut fb,&models,Body::Earth,&camera(),time,true,false,4);
            images.push(fb.buffer.clone());
        }
        for i in 0..images.len(){for j in i+1..images.len(){assert_ne!(images[i],images[j]);}}
        render(&mut fb,&models,Body::Earth,&camera(),0.0,true,false,4);
        assert_eq!(fb.buffer,images[0]);
        // At fixed camera/time the central body uses the same projection.
        // Removing only the moon mesh must leave pixels hidden behind Earth unchanged.
        let mut no_moon=Models::load().unwrap();no_moon.moon.faces.clear();
        render(&mut fb,&no_moon,Body::Earth,&camera(),0.0,true,false,4);
        assert_ne!(fb.buffer,images[0]);
        assert_eq!(fb.buffer[100*240+120],images[0][100*240+120]);
    }
    #[test]
    fn wireframe_toggle_restores_the_exact_shaded_surface() {
        let models=Models::load().unwrap();let mut fb=Framebuffer::new(192,160);
        render(&mut fb,&models,Body::Mogo,&camera(),0.0,true,false,4);let surface=fb.buffer.clone();
        render(&mut fb,&models,Body::Mogo,&camera(),0.0,true,true,4);assert_ne!(fb.buffer,surface);
        render(&mut fb,&models,Body::Mogo,&camera(),0.0,true,false,4);assert_eq!(fb.buffer,surface);
    }
}
