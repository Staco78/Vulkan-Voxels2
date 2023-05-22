use std::{
    fs::{self, OpenOptions},
    sync::{atomic::Ordering, Mutex},
    time::Instant,
};

use anyhow::Result;

use crate::gui;

#[derive(Debug)]
struct DataFrame {
    time: Instant,
    fps: f32,

    pub created_chunks_total: usize,
    pub generated_chunks_total: usize,
    pub meshed_chunks_total: usize,
    pub created_chunks: usize,
    pub generated_chunks: usize,
    pub meshed_chunks: usize,

    pub waiting_for_generate_chunks: usize,
    pub waiting_for_mesh_chunks: usize,

    pub loaded_chunks: usize,
    pub loaded_regions: usize,
}

impl From<&gui::Data> for DataFrame {
    fn from(data: &gui::Data) -> Self {
        Self {
            time: Instant::now(),
            fps: data.fps_calculator.fps(),

            created_chunks_total: data.created_chunks_total.load(Ordering::Relaxed),
            generated_chunks_total: data.generated_chunks_total.load(Ordering::Relaxed),
            meshed_chunks_total: data.meshed_chunks_total.load(Ordering::Relaxed),
            created_chunks: data.created_chunks.load(Ordering::Relaxed),
            generated_chunks: data.generated_chunks.load(Ordering::Relaxed),
            meshed_chunks: data.meshed_chunks.load(Ordering::Relaxed),

            waiting_for_generate_chunks: data.waiting_for_generate_chunks.load(Ordering::Relaxed),
            waiting_for_mesh_chunks: data.waiting_for_mesh_chunks.load(Ordering::Relaxed),

            loaded_chunks: data.loaded_chunks.load(Ordering::Relaxed),
            loaded_regions: data.loaded_regions.load(Ordering::Relaxed),
        }
    }
}

static DATA: Mutex<Vec<DataFrame>> = Mutex::new(Vec::new());

pub fn append(gui_data: &gui::Data) {
    let frame = gui_data.into();
    DATA.lock().expect("Mutex poisoned").push(frame);
}

pub fn end() {
    let data = DATA.lock().expect("Mutex poisoned");
    print_infos_fps(&data);
    print_infos_chunks(&data);
    emit_csv(&data).expect("Csv emit failed");
}

fn print_infos_fps(data: &[DataFrame]) {
    let average_fps = data.iter().fold(0., |acc, e| acc + e.fps) / data.len() as f32;
    let mut sorted = Vec::from_iter(data);
    sorted.sort_by(|&a, &b| a.fps.total_cmp(&b.fps));
    let average_low_fps = sorted
        .iter()
        .take(data.len() / 10)
        .fold(0., |acc, e| acc + e.fps)
        / (data.len() / 10) as f32;

    println!("Average fps: {}", average_fps);
    println!("Average low fps: {}", average_low_fps);
}

fn print_infos_chunks(data: &[DataFrame]) {
    let last = data.last().expect("Data is empty");

    println!("Total created chunks: {}", last.created_chunks_total);
    println!("Total generated chunks: {}", last.generated_chunks_total);
    println!("Total meshed chunks: {}", last.meshed_chunks_total);

    println!(
        "Chunks creation rate: {}/s",
        last.created_chunks_total as f32 / 60.
    );
    println!(
        "Chunks generation rate: {}/s",
        last.generated_chunks_total as f32 / 60.
    );
    println!(
        "Chunks meshing rate: {}/s",
        last.meshed_chunks_total as f32 / 60.
    );
}

fn emit_csv(data: &[DataFrame]) -> Result<()> {
    let dir = "bench_results";
    fs::create_dir_all(dir)?;
    let path = format!(
        "{dir}/{}_{}.csv",
        chrono::Local::now().format("%F-%H-%M-%S"),
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
    let file = OpenOptions::new().create_new(true).write(true).open(path)?;
    let mut writer = csv::Writer::from_writer(&file);

    writer.write_record([
        "time",
        "fps",
        "created_chunks_total",
        "generated_chunks_total",
        "meshed_chunks_total",
        "created_chunks",
        "generated_chunks",
        "meshed_chunks",
        "waiting_for_generate_chunks",
        "waiting_for_mesh_chunks",
        "loaded_chunks",
        "loaded_regions",
    ])?;
    for DataFrame {
        time,
        fps,
        created_chunks_total,
        generated_chunks_total,
        meshed_chunks_total,
        created_chunks,
        generated_chunks,
        meshed_chunks,
        waiting_for_generate_chunks,
        waiting_for_mesh_chunks,
        loaded_chunks,
        loaded_regions,
    } in data
    {
        let time = time.duration_since(data[0].time).as_secs_f32();
        writer.serialize((
            time,
            fps,
            created_chunks_total,
            generated_chunks_total,
            meshed_chunks_total,
            created_chunks,
            generated_chunks,
            meshed_chunks,
            waiting_for_generate_chunks,
            waiting_for_mesh_chunks,
            loaded_chunks,
            loaded_regions,
        ))?;
    }
    writer.flush()?;
    Ok(())
}
