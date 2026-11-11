//! Synchronous GPU-complete throughput; intentionally separate from displayed FPS.
use crate::{app, canvas_fixture, jobs, view};

#[test]
#[ignore = "release GPU throughput benchmark; requires Metal access and an explicit copied CODECALLNAVIGATOR_DATA_DIR"]
fn canvas_gpu_frame_completion() -> anyhow::Result<()> {
    assert!(!std::hint::black_box(cfg!(debug_assertions)), "Run with --release");
    assert!(
        std::env::var_os("CODECALLNAVIGATOR_DATA_DIR").is_some(),
        "Use a copied profiling workspace"
    );
    use rune::render::Gpu;
    let gpu = Gpu::try_new(Gpu::instance(), None).map_err(|error| anyhow::anyhow!(error))?;
    println!(
        "Synchronous offscreen GPU-complete throughput; NOT displayed FPS. adapter={:?}",
        gpu.adapter.get_info()
    );
    for name in ["dense", "saved"] {
        for scale in [1.0, 2.0] {
            measure(&gpu, name, scale)?;
        }
    }
    Ok(())
}

fn measure(gpu: &rune::render::Gpu, name: &str, scale: f32) -> anyhow::Result<()> {
    use rune::render::{KIND_SEGMENT, OFFSCREEN_FORMAT, Offscreen, Renderer};
    use std::time::Instant;
    let mut rt = runtime(name, scale)?;
    let origin = rt.state.nav.camera.pan;
    let zoom = rt.state.nav.camera.zoom;
    let nodes = rt.state.graph_view.nodes.len();
    let edges = rt.state.graph_view.edges.len();
    let size = ((1900.0 * scale) as u32, (1100.0 * scale) as u32);
    let format = if rt.srgb_blend() {
        OFFSCREEN_FORMAT.remove_srgb_suffix()
    } else {
        OFFSCREEN_FORMAT
    };
    let target = Offscreen::with_format(&gpu.device, size.0, size.1, format);
    let mut renderer = Renderer::new(&gpu.device, format);
    renderer.set_srgb_blend(rt.srgb_blend());
    let mut update = Vec::new();
    let mut encode = Vec::new();
    let mut complete = Vec::new();
    let mut segments = Vec::new();
    let mut instances = Vec::new();
    for frame in 0..40 {
        let began = Instant::now();
        rt.state.nav.camera.pan = [
            origin[0] + (frame as f32 * 0.2).sin() * 180.0,
            origin[1] + (frame as f32 * 0.2).cos() * 90.0,
        ];
        rt.invalidate();
        rt.frame(1.0 / 60.0);
        let update_ms = began.elapsed().as_secs_f64() * 1000.0;
        renderer.upload_images(&gpu.device, &gpu.queue, &mut rt.images);
        renderer.prepare_surfaces(&gpu.device, &gpu.queue, &rt.surfaces, size);
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        renderer.prepare(
            &gpu.device,
            &gpu.queue,
            &mut rt.text.atlas,
            &rt.instances,
            size,
        );
        renderer.draw(&mut encoder, &target.view, rt.clear_color());
        renderer.submit(&gpu.queue, encoder);
        let encoded_ms = began.elapsed().as_secs_f64() * 1000.0;
        gpu.device.poll(wgpu::PollType::wait_indefinitely())?;
        let complete_ms = began.elapsed().as_secs_f64() * 1000.0;
        if frame >= 10 {
            update.push(update_ms);
            encode.push(encoded_ms - update_ms);
            complete.push(complete_ms);
            segments.push(
                rt.instances
                    .iter()
                    .filter(|i| i.params[1] == KIND_SEGMENT)
                    .count(),
            );
            instances.push(rt.instances.len());
        }
    }
    println!(
        "{}",
        serde_json::json!({"workload":name,"scale":scale,"size_px":[size.0,size.1],"nodes":nodes,"edges":edges,"zoom":zoom,"warmup":10,"frames":30,"update_ms":stats(update),"upload_encode_ms":stats(encode),"gpu_complete_frame_ms":stats(complete),"mean_segments":segments.iter().sum::<usize>()/segments.len(),"mean_instances":instances.iter().sum::<usize>()/instances.len()})
    );
    Ok(())
}

fn stats(mut times: Vec<f64>) -> serde_json::Value {
    times.sort_by(f64::total_cmp);
    serde_json::json!({"median":times[times.len()/2],"p95":times[times.len()*95/100],"max":times[times.len()-1]})
}

fn runtime(name: &str, scale: f32) -> anyhow::Result<rune::Runtime<app::State>> {
    let mut state = app::State::new();
    match name {
        "dense" => canvas_fixture::install(&mut state),
        _ => canvas_fixture::saved(&mut state),
    }
    anyhow::ensure!(state.warning.is_empty(), "{}", state.warning);
    anyhow::ensure!(!state.graph_view.edges.is_empty(), "empty graph");
    let mut rt = rune::Runtime::new(
        state,
        view::view,
        (1900.0, 1100.0),
        rune::Env::new(true, scale),
    );
    rt.poll = Some(jobs::poll);
    rt.settle_all();
    rt.frame(0.0);
    Ok(rt)
}
