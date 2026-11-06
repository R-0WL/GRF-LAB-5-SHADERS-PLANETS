use minifb::{Key, KeyRepeat, Window, WindowOptions};
use crate::cosmos::{Background, Backgrounds};
use std::{error::Error,path::{Path,PathBuf},time::{Duration,Instant}};
use crate::{framebuffer::Framebuffer, scene::{self,Body,Models},hud,HEIGHT,WIDTH,ROTATION_SPEED};

struct Options {
    width:usize,height:usize,body:Body,layers:u8,time:f32,output:PathBuf,
    background:Background,fluid:bool,sky_time:f32,
    render_only:bool,sheet:bool,screenshots:bool,paused:bool,hud:bool,extras:bool,frames:Option<usize>,
}
const HELP:&str="LAB5 / Mundos de ciencia ficcion
cargo run --release -- [opciones]
--escena mogo|cybertron|praxis|sirio|sol|starkiller|tierra
--capas 1..4        Capas acumulativas (4 = shader final)
--comparar-capas    Exporta una lamina 2x2 sin abrir ventana
--solo-render       Exporta PNG sin ventana
--screenshots       Exporta los siete cuerpos y sus siete laminas
--salida ruta.png   Destino del PNG (screenshots/captura.png)
--ancho 1280 --alto 960 --tiempo 0
--pausado --sin-titulos --sin-extras --fotogramas N
--fondo clasico|nebulosas|galaxias (nebulosas por defecto)
--fondo-estatico / --fondo-fluido (fluido por defecto)
--tiempo-fondo N     Tiempo reproducible del fondo; por defecto usa --tiempo
B cambia fondo | F pausa/reanuda el fondo de forma independiente
1 Mogo | 2 Cybertron | 3 Praxis | 4 Sirio | 5 Sol | 6 Starkiller | 7 Tierra
Tab siguiente | C capas | V comparar | P exporta lamina | S captura visible
Flechas orbitar | Espacio pausa | E extras | H malla | T titulos | R reiniciar | Esc salir";
fn parse()->Result<Option<Options>,Box<dyn Error>> {
    let mut o=Options{width:WIDTH,height:HEIGHT,body:Body::Mogo,layers:4,time:0.0,
        output:"screenshots/captura.png".into(),render_only:false,sheet:false,screenshots:false,
        paused:false,hud:true,extras:true,frames:None,background:Background::Nebula,fluid:true,sky_time:0.0};
    let mut explicit_sky_time=false;
    let mut args=std::env::args().skip(1);
    while let Some(arg)=args.next(){
        let mut value=|| args.next().ok_or_else(||format!("Falta valor para {arg}"));
        match arg.as_str(){
            "--help"|"--ayuda"|"-h"=>{println!("{HELP}");return Ok(None)},
            "--escena"|"--scene"=>o.body=Body::parse(&value()?).ok_or("Escena invalida; usa --help")?,
            "--fondo"|"--background"=>o.background=Background::parse(&value()?).ok_or("Fondo invalido: clasico, nebulosas o galaxias")?,
            "--fondo-estatico"=>o.fluid=false,
            "--fondo-fluido"=>o.fluid=true,
            "--tiempo-fondo"=>{o.sky_time=value()?.parse()?;explicit_sky_time=true;},
            "--capas"|"--layers"=>o.layers=value()?.parse()?,
            "--ancho"|"--width"=>o.width=value()?.parse()?,
            "--alto"|"--height"=>o.height=value()?.parse()?,
            "--tiempo"|"--time"=>o.time=value()?.parse()?,
            "--salida"|"--output"=>o.output=value()?.into(),
            "--fotogramas"|"--frames"=>o.frames=Some(value()?.parse()?),
            "--solo-render"|"--render-only"=>o.render_only=true,
            "--comparar-capas"|"--layer-sheet"=>{o.sheet=true;o.render_only=true},
            "--screenshots"=>{o.screenshots=true;o.render_only=true},
            "--pausado"|"--paused"=>o.paused=true,
            "--sin-titulos"|"--no-hud"=>o.hud=false,
            "--sin-extras"=>o.extras=false,
            _=>return Err(format!("Opcion desconocida: {arg}; usa --help").into()),
        }
    }
    if !(160..=4096).contains(&o.width)||!(160..=4096).contains(&o.height){return Err("Dimensiones: 160..4096".into())}
    if !(1..=4).contains(&o.layers){return Err("--capas debe estar entre 1 y 4".into())}
    if !explicit_sky_time {o.sky_time=o.time;}
    if !o.sky_time.is_finite(){return Err("--tiempo-fondo debe ser finito".into())}
    if !o.time.is_finite(){return Err("--tiempo debe ser finito".into())}
    if o.frames==Some(0){return Err("--fotogramas debe ser mayor que cero".into())}
    Ok(Some(o))
}
fn framebuffer(w:usize,h:usize)->Framebuffer{
    Framebuffer::new(w,h)
}
fn save(fb:&Framebuffer,path:&Path)->Result<(),Box<dyn Error>>{
    if let Some(parent)=path.parent().filter(|p|!p.as_os_str().is_empty()){std::fs::create_dir_all(parent)?}
    fb.save_png(path)?;println!("Guardado: {}",path.display());Ok(())
}
fn sheet(fb:&mut Framebuffer,models:&Models,o:&Options,camera:&crate::camera::Camera,hologram:bool,backgrounds:&mut Backgrounds){
    fb.clear();
    let w=fb.width/2;let h=fb.height/2;
    for i in 0..4 {
        let pw=if i%2==0 {w} else {fb.width-w};
        let ph=if i/2==0 {h} else {fb.height-h};
        let mut panel=framebuffer(pw,ph);
        panel.set_background(backgrounds.render(pw,ph,o.background,o.sky_time));
        scene::render(&mut panel,models,o.body,camera,o.time,o.extras,hologram,i as u8+1);
        hud::heading(&mut panel,o.body.name(),i as u8+1);
        let ox=(i%2)*w;let oy=(i/2)*h;
        for y in 0..ph {let start=(oy+y)*fb.width+ox;fb.buffer[start..start+pw].copy_from_slice(&panel.buffer[y*pw..(y+1)*pw]);}
    }
    let fw=fb.width;let fh=fb.height;
    hud::rect(fb,w,0,1,fh,0x465269);hud::rect(fb,0,h,fw,1,0x465269);
}
fn draw(fb:&mut Framebuffer,models:&Models,o:&Options,camera:&crate::camera::Camera,hologram:bool,backgrounds:&mut Backgrounds){
    if o.sheet {sheet(fb,models,o,camera,hologram,backgrounds);return}
    fb.set_background(backgrounds.render(fb.width,fb.height,o.background,o.sky_time));
    scene::render(fb,models,o.body,camera,o.time,o.extras,hologram,o.layers);
    if o.hud {
        hud::heading(fb,o.body.name(),o.layers);
        if fb.width>=640 {
            let y=fb.height.saturating_sub(24);
            hud::caption(fb,"1-7 CUERPOS / C CAPAS / V COMPARAR / B FONDO / F FLUJO / S PNG",16,y);
        }
    }
}
pub fn run()->Result<(),Box<dyn Error>>{
    let Some(mut o)=parse()? else{return Ok(())};
    let models=Models::load()?;
    let mut backgrounds=Backgrounds::default();
    let mut fb=framebuffer(o.width,o.height);
    let mut camera=scene::camera();
    if o.screenshots {
        let folder=o.output.parent().filter(|p|!p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        let folder=folder.to_path_buf();
        for body in Body::ALL {
            o.body=body;o.layers=4;o.sheet=false;
            draw(&mut fb,&models,&o,&camera,false,&mut backgrounds);
            save(&fb,&folder.join(format!("{}.png",body.slug())))?;
            sheet(&mut fb,&models,&o,&camera,false,&mut backgrounds);
            save(&fb,&folder.join(format!("capas-{}.png",body.slug())))?;
        }
        return Ok(())
    }
    draw(&mut fb,&models,&o,&camera,false,&mut backgrounds);
    if o.render_only {return save(&fb,&o.output)}
    let mut window=Window::new("LAB5",o.width,o.height,WindowOptions::default())?;
    let mut last=Instant::now();let mut count=0;let mut hologram=false;
    while window.is_open()&&!window.is_key_down(Key::Escape){
        let previous=(o.body,o.layers,o.time,o.paused,o.extras,o.hud,o.sheet,hologram,camera.eye,o.background,o.fluid,o.sky_time);
        let now=Instant::now();let dt=(now-last).as_secs_f32().min(0.1);last=now;
        for (key,body) in [Key::Key1,Key::Key2,Key::Key3,Key::Key4,Key::Key5,Key::Key6,Key::Key7].into_iter().zip(Body::ALL){
            if window.is_key_pressed(key,KeyRepeat::No){o.body=body;o.time=0.0;camera=scene::camera();}
        }
        if window.is_key_pressed(Key::Tab,KeyRepeat::No){let i=Body::ALL.iter().position(|b|*b==o.body).unwrap();o.body=Body::ALL[(i+1)%7];o.time=0.0;camera=scene::camera();}
        for (key,flag) in [(Key::Space,&mut o.paused),(Key::E,&mut o.extras),(Key::H,&mut hologram),(Key::T,&mut o.hud),(Key::V,&mut o.sheet)]{
            if window.is_key_pressed(key,KeyRepeat::No){*flag=!*flag;}
        }
        if window.is_key_pressed(Key::C,KeyRepeat::No){o.layers=o.layers%4+1;}
        if window.is_key_pressed(Key::R,KeyRepeat::No){o.time=0.0;camera=scene::camera();}else if !o.paused{o.time+=dt;}
        if window.is_key_pressed(Key::B,KeyRepeat::No){o.background=o.background.next();}
        if window.is_key_pressed(Key::F,KeyRepeat::No){o.fluid=!o.fluid;}
        if window.is_key_pressed(Key::R,KeyRepeat::No){o.sky_time=0.0;}else if o.fluid && o.background!=Background::Classic {o.sky_time+=dt;}
        let speed=ROTATION_SPEED*60.0*dt;
        for (key,yaw,pitch) in [(Key::Left,-speed,0.0),(Key::Right,speed,0.0),(Key::Up,0.0,-speed),(Key::Down,0.0,speed)]{
            if window.is_key_down(key){camera.orbit(yaw,pitch);}
        }
        let current=(o.body,o.layers,o.time,o.paused,o.extras,o.hud,o.sheet,hologram,camera.eye,o.background,o.fluid,o.sky_time);
        if count==0 || previous!=current {draw(&mut fb,&models,&o,&camera,hologram,&mut backgrounds);}
        window.set_title(&format!("{} | Capas 1-{} | {} | Fondo {} / {} | B fondo | F flujo | C capas | V comparar | S PNG",o.body.name(),o.layers,if o.paused{"Pausado"}else{"Animando"},o.background.name(),if o.background==Background::Classic {"estatico"} else if o.fluid {"fluido"} else {"congelado"}));
        if window.is_key_pressed(Key::S,KeyRepeat::No){save(&fb,&o.output)?;}
        if window.is_key_pressed(Key::P,KeyRepeat::No){
            let mut comparison=framebuffer(o.width,o.height);sheet(&mut comparison,&models,&o,&camera,hologram,&mut backgrounds);
            let path=o.output.with_file_name(format!("capas-{}.png",o.body.slug()));save(&comparison,&path)?;
        }
        window.update_with_buffer(&fb.buffer,o.width,o.height)?;
        count+=1;if o.frames.is_some_and(|n|count>=n){break}
        std::thread::sleep(Duration::from_millis(8));
    }
    Ok(())
}
