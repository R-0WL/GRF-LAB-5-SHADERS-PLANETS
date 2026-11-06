use std::{path::PathBuf,process::Command};
fn binary()->Command{Command::new(env!("CARGO_BIN_EXE_Graficas_LAB5"))}
#[test]
fn invalid_cli_values_fail_before_rendering(){
    for args in [vec!["--capas","0"],vec!["--capas","5"],vec!["--tiempo","NaN"],vec!["--ancho","0"],vec!["--escena","desconocido"],vec!["--salida"],vec!["--fotogramas","0"],vec!["--fondo","incorrecto"],vec!["--tiempo-fondo","NaN"]]{
        let result=binary().args(args).output().unwrap();assert!(!result.status.success());
    }
}
#[test]
fn headless_export_writes_valid_png_and_all_four_odd_sized_panels(){
    let dir=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join(format!("cli-test-{}",std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path=dir.join("layers.png");
    let output=binary().args(["--escena","sol","--comparar-capas","--ancho","321","--alto","241","--salida"]).arg(&path).output().unwrap();
    assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));
    let decoder=png::Decoder::new(std::fs::File::open(&path).unwrap());let mut reader=decoder.read_info().unwrap();
    let mut pixels=vec![0;reader.output_buffer_size()];let info=reader.next_frame(&mut pixels).unwrap();
    assert_eq!((info.width,info.height),(321,241));
    // Every quadrant has a painted sphere below its title band.
    let w=321_usize;
    for (x,y) in [(80,65),(240,65),(80,185),(240,185)]{
        let i=(y*w+x)*3;assert!(pixels[i..i+3].iter().any(|p|*p>50));
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn exported_sky_time_is_reproducible_and_independent_of_planet_time(){
    let dir=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join(format!("sky-test-{}",std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut images=Vec::new();
    for (index,time,flag) in [(0,"0","--fondo-estatico"),(1,"4","--fondo-estatico"),(2,"0","--fondo-fluido")] {
        let path=dir.join(format!("sky-{index}.png"));
        let output=binary().args(["--solo-render","--escena","tierra","--ancho","160","--alto","160","--fondo","galaxias","--tiempo","0","--tiempo-fondo",time,flag,"--salida"]).arg(&path).output().unwrap();
        assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));
        images.push(std::fs::read(path).unwrap());
    }
    assert_ne!(images[0],images[1]);
    assert_eq!(images[0],images[2]);
    std::fs::remove_dir_all(dir).unwrap();
}
