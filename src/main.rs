use anyhow::Result;
use axum::{routing::get, Router};
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};
use opentelemetry::{global, metrics::{Meter, Unit}, KeyValue};
use opentelemetry_sdk::{
    metrics::SdkMeterProvider,
    Resource,
};
use prometheus::{Encoder, TextEncoder};
use sysinfo::{System, RefreshKind, CpuRefreshKind, MemoryRefreshKind, Pid};
use uuid::Uuid;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time;

/// System metrics collector that tracks CPU and memory usage
struct SystemMetrics {
    system: Arc<Mutex<System>>,
    cpu_time_seconds: Arc<Mutex<f64>>,
    last_cpu_sample: Arc<Mutex<Instant>>,
    self_pid: Pid,
}

impl SystemMetrics {
    fn new() -> Self {
        let self_pid = Pid::from(std::process::id() as usize);
        let mut system = System::new_with_specifics(
            RefreshKind::new()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        system.refresh_process(self_pid);

        Self {
            system: Arc::new(Mutex::new(system)),
            cpu_time_seconds: Arc::new(Mutex::new(0.0)),
            last_cpu_sample: Arc::new(Mutex::new(Instant::now())),
            self_pid,
        }
    }

    fn get_cpu_usage(&self) -> f64 {
        let mut system = self.system.lock().unwrap();
        system.refresh_cpu();
        let usage = system.global_cpu_info().cpu_usage() as f64;
        let num_cpus = system.cpus().len() as f64;
        drop(system);

        let mut last_sample = self.last_cpu_sample.lock().unwrap();
        let elapsed = last_sample.elapsed().as_secs_f64().min(30.0);
        *last_sample = Instant::now();
        drop(last_sample);

        *self.cpu_time_seconds.lock().unwrap() += (usage / 100.0) * elapsed * num_cpus;

        usage
    }

    fn get_cpu_time_seconds(&self) -> f64 {
        *self.cpu_time_seconds.lock().unwrap()
    }

    fn get_self_cpu_usage(&self) -> f64 {
        let mut system = self.system.lock().unwrap();
        system.refresh_process(self.self_pid);
        system.process(self.self_pid)
            .map(|p| p.cpu_usage() as f64)
            .unwrap_or(0.0)
    }

    fn get_self_memory_bytes(&self) -> u64 {
        let mut system = self.system.lock().unwrap();
        system.refresh_process(self.self_pid);
        system.process(self.self_pid)
            .map(|p| p.memory())
            .unwrap_or(0)
    }

    fn get_memory_usage_percent(&self) -> f64 {
        let mut system = self.system.lock().unwrap();
        system.refresh_memory();
        let used = system.used_memory() as f64;
        let total = system.total_memory() as f64;
        (used / total) * 100.0
    }

    fn get_memory_used_bytes(&self) -> u64 {
        let mut system = self.system.lock().unwrap();
        system.refresh_memory();
        system.used_memory()
    }

    fn get_memory_total_bytes(&self) -> u64 {
        let mut system = self.system.lock().unwrap();
        system.refresh_memory();
        system.total_memory()
    }
}

// --- Aranet4 CO2 monitor ---

const ARANET4_READINGS_UUID: &str = "f0cd3001-95da-4f4b-9ac8-aa55d312af0c";

#[derive(Clone)]
struct Aranet4Reading {
    device_name: String,
    co2_ppm: u16,
    temperature_c: f32,
    pressure_hpa: f32,
    humidity_pct: u8,
    battery_pct: u8,
    interval_secs: u16,
    age_secs: u16,
}

struct Aranet4Metrics {
    latest: Arc<Mutex<Option<Aranet4Reading>>>,
}

impl Aranet4Metrics {
    fn new() -> Self {
        Self { latest: Arc::new(Mutex::new(None)) }
    }

    fn update(&self, reading: Aranet4Reading) {
        *self.latest.lock().unwrap() = Some(reading);
    }

    fn get(&self) -> Option<Aranet4Reading> {
        self.latest.lock().unwrap().clone()
    }
}

fn parse_aranet4(device_name: String, data: &[u8]) -> Option<Aranet4Reading> {
    if data.len() < 13 {
        return None;
    }
    Some(Aranet4Reading {
        device_name,
        co2_ppm: u16::from_le_bytes([data[0], data[1]]),
        temperature_c: u16::from_le_bytes([data[2], data[3]]) as f32 / 20.0,
        pressure_hpa: u16::from_le_bytes([data[4], data[5]]) as f32 / 10.0,
        humidity_pct: data[6],
        battery_pct: data[7],
        interval_secs: u16::from_le_bytes([data[9], data[10]]),
        age_secs: u16::from_le_bytes([data[11], data[12]]),
    })
}

async fn find_aranet4(adapter: &Adapter) -> anyhow::Result<(Peripheral, String)> {
    adapter.start_scan(ScanFilter::default()).await?;
    time::sleep(Duration::from_secs(10)).await;
    adapter.stop_scan().await?;

    for peripheral in adapter.peripherals().await? {
        if let Some(props) = peripheral.properties().await? {
            if let Some(name) = props.local_name.filter(|n| n.starts_with("Aranet4")) {
                return Ok((peripheral, name));
            }
        }
    }
    anyhow::bail!("Aranet4 not found during scan")
}

async fn try_collect_aranet4(metrics: &Arc<Aranet4Metrics>) -> anyhow::Result<u64> {
    let manager = Manager::new().await?;
    let adapter = manager.adapters().await?
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No Bluetooth adapter found"))?;

    let (device, device_name) = find_aranet4(&adapter).await?;
    device.connect().await?;
    device.discover_services().await?;

    let readings_uuid = Uuid::parse_str(ARANET4_READINGS_UUID)?;
    let characteristic = device.characteristics()
        .into_iter()
        .find(|c| c.uuid == readings_uuid)
        .ok_or_else(|| anyhow::anyhow!("Aranet4 readings characteristic not found"))?;

    let data = device.read(&characteristic).await?;
    device.disconnect().await.ok();

    let reading = parse_aranet4(device_name, &data)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse Aranet4 data ({} bytes)", data.len()))?;

    let wait_secs = (reading.interval_secs.saturating_sub(reading.age_secs) as u64).max(5);
    metrics.update(reading);
    Ok(wait_secs)
}

async fn aranet4_collection_loop(metrics: Arc<Aranet4Metrics>) {
    loop {
        match try_collect_aranet4(&metrics).await {
            Ok(wait_secs) => time::sleep(Duration::from_secs(wait_secs)).await,
            Err(e) => {
                eprintln!("Aranet4 error: {e}");
                time::sleep(Duration::from_secs(30)).await;
            }
        }
    }
}

fn register_aranet4_metrics(meter: &Meter, aranet4: Arc<Aranet4Metrics>) -> Result<()> {
    let hostname = hostname::get().unwrap().to_string_lossy().to_string();

    let a = aranet4.clone();
    let h = hostname.clone();
    let _co2 = meter
        .f64_observable_gauge("environment.co2")
        .with_description("CO2 concentration")
        .with_unit(Unit::new("ppm"))
        .with_callback(move |observer| {
            if let Some(r) = a.get() {
                observer.observe(r.co2_ppm as f64, &[
                    KeyValue::new("host.name", h.clone()),
                    KeyValue::new("device", r.device_name.clone()),
                ]);
            }
        })
        .init();

    let a = aranet4.clone();
    let h = hostname.clone();
    let _temp = meter
        .f64_observable_gauge("environment.temperature")
        .with_description("Ambient temperature")
        .with_unit(Unit::new("Cel"))
        .with_callback(move |observer| {
            if let Some(r) = a.get() {
                observer.observe(r.temperature_c as f64, &[
                    KeyValue::new("host.name", h.clone()),
                    KeyValue::new("device", r.device_name.clone()),
                ]);
            }
        })
        .init();

    let a = aranet4.clone();
    let h = hostname.clone();
    let _pressure = meter
        .f64_observable_gauge("environment.pressure")
        .with_description("Atmospheric pressure")
        .with_unit(Unit::new("hPa"))
        .with_callback(move |observer| {
            if let Some(r) = a.get() {
                observer.observe(r.pressure_hpa as f64, &[
                    KeyValue::new("host.name", h.clone()),
                    KeyValue::new("device", r.device_name.clone()),
                ]);
            }
        })
        .init();

    let a = aranet4.clone();
    let h = hostname.clone();
    let _humidity = meter
        .f64_observable_gauge("environment.humidity")
        .with_description("Relative humidity")
        .with_unit(Unit::new("%"))
        .with_callback(move |observer| {
            if let Some(r) = a.get() {
                observer.observe(r.humidity_pct as f64, &[
                    KeyValue::new("host.name", h.clone()),
                    KeyValue::new("device", r.device_name.clone()),
                ]);
            }
        })
        .init();

    let a = aranet4.clone();
    let h = hostname;
    let _battery = meter
        .f64_observable_gauge("environment.aranet4.battery")
        .with_description("Aranet4 battery level")
        .with_unit(Unit::new("%"))
        .with_callback(move |observer| {
            if let Some(r) = a.get() {
                observer.observe(r.battery_pct as f64, &[
                    KeyValue::new("host.name", h.clone()),
                    KeyValue::new("device", r.device_name.clone()),
                ]);
            }
        })
        .init();

    Ok(())
}

/// Initialize OpenTelemetry metrics with Prometheus exporter
fn init_metrics_with_prometheus() -> Result<(SdkMeterProvider, prometheus::Registry)> {
    let registry = prometheus::Registry::new();
    let exporter = opentelemetry_prometheus::exporter()
        .with_registry(registry.clone())
        .build()?;

    let provider = SdkMeterProvider::builder()
        .with_reader(exporter)
        .with_resource(Resource::new(vec![
            KeyValue::new("service.name", "telemetry-service"),
            KeyValue::new("service.version", "0.1.0"),
        ]))
        .build();

    global::set_meter_provider(provider.clone());

    Ok((provider, registry))
}

/// Register observable gauges for CPU and memory metrics
fn register_metrics(meter: &Meter, metrics: Arc<SystemMetrics>) -> Result<()> {
    let hostname = hostname::get()
        .unwrap()
        .to_string_lossy()
        .to_string();

    // CPU usage gauge
    let cpu_metrics = metrics.clone();
    let hostname_clone = hostname.clone();
    let _cpu_gauge = meter
        .f64_observable_gauge("system.cpu.utilization")
        .with_description("CPU utilization percentage")
        .with_unit(Unit::new("%"))
        .with_callback(move |observer| {
            let usage = cpu_metrics.get_cpu_usage();
            observer.observe(
                usage,
                &[KeyValue::new("host.name", hostname_clone.clone())],
            );
        })
        .init();

    // CPU time counter (accumulated seconds = usage_fraction * elapsed * num_cpus)
    let cpu_time_metrics = metrics.clone();
    let hostname_clone = hostname.clone();
    let _cpu_time_counter = meter
        .f64_observable_counter("system.cpu.time")
        .with_description("Cumulative CPU time consumed across all cores")
        .with_unit(Unit::new("s"))
        .with_callback(move |observer| {
            let time = cpu_time_metrics.get_cpu_time_seconds();
            observer.observe(
                time,
                &[KeyValue::new("host.name", hostname_clone.clone())],
            );
        })
        .init();

    // Memory usage percentage gauge
    let mem_percent_metrics = metrics.clone();
    let hostname_clone = hostname.clone();
    let _mem_percent_gauge = meter
        .f64_observable_gauge("system.memory.utilization")
        .with_description("Memory utilization percentage")
        .with_unit(Unit::new("%"))
        .with_callback(move |observer| {
            let usage = mem_percent_metrics.get_memory_usage_percent();
            observer.observe(
                usage,
                &[KeyValue::new("host.name", hostname_clone.clone())],
            );
        })
        .init();

    // Memory used bytes gauge
    let mem_used_metrics = metrics.clone();
    let hostname_clone = hostname.clone();
    let _mem_used_gauge = meter
        .u64_observable_gauge("system.memory.usage")
        .with_description("Memory used in bytes")
        .with_unit(Unit::new("By"))
        .with_callback(move |observer| {
            let used = mem_used_metrics.get_memory_used_bytes();
            observer.observe(
                used,
                &[KeyValue::new("host.name", hostname_clone.clone())],
            );
        })
        .init();

    // Memory total bytes gauge
    let mem_total_metrics = metrics.clone();
    let hostname_clone = hostname.clone();
    let _mem_total_gauge = meter
        .u64_observable_gauge("system.memory.total")
        .with_description("Total memory in bytes")
        .with_unit(Unit::new("By"))
        .with_callback(move |observer| {
            let total = mem_total_metrics.get_memory_total_bytes();
            observer.observe(
                total,
                &[KeyValue::new("host.name", hostname_clone.clone())],
            );
        })
        .init();

    // Process CPU usage gauge
    let proc_cpu_metrics = metrics.clone();
    let hostname_clone = hostname.clone();
    let _proc_cpu_gauge = meter
        .f64_observable_gauge("process.cpu.usage")
        .with_description("CPU usage of the telemetry-service process")
        .with_unit(Unit::new("%"))
        .with_callback(move |observer| {
            let usage = proc_cpu_metrics.get_self_cpu_usage();
            observer.observe(
                usage,
                &[KeyValue::new("host.name", hostname_clone.clone())],
            );
        })
        .init();

    // Process memory usage gauge
    let proc_mem_metrics = metrics.clone();
    let hostname_clone = hostname;
    let _proc_mem_gauge = meter
        .u64_observable_gauge("process.memory.usage")
        .with_description("RSS memory usage of the telemetry-service process")
        .with_unit(Unit::new("By"))
        .with_callback(move |observer| {
            let bytes = proc_mem_metrics.get_self_memory_bytes();
            observer.observe(
                bytes,
                &[KeyValue::new("host.name", hostname_clone.clone())],
            );
        })
        .init();

    Ok(())
}

/// Prometheus metrics endpoint handler
async fn metrics_handler(
    registry: Arc<prometheus::Registry>,
) -> Result<String, (axum::http::StatusCode, String)> {
    let metric_families = registry.gather();
    let encoder = TextEncoder::new();
    let mut buffer = Vec::new();
    
    encoder
        .encode(&metric_families, &mut buffer)
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to encode metrics: {}", e),
            )
        })?;

    String::from_utf8(buffer).map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to convert metrics to string: {}", e),
        )
    })
}

/// Health check endpoint
async fn health_handler() -> &'static str {
    "OK"
}

/// Manual metrics collection and logging loop
async fn metrics_logging_loop(metrics: Arc<SystemMetrics>, aranet4: Arc<Aranet4Metrics>) {
    let hostname = hostname::get()
        .unwrap()
        .to_string_lossy()
        .to_string();

    let mut interval = time::interval(Duration::from_secs(5));
    
    loop {
        interval.tick().await;
        
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let cpu_usage = metrics.get_cpu_usage();
        let cpu_time = metrics.get_cpu_time_seconds();
        let mem_usage_percent = metrics.get_memory_usage_percent();
        let mem_used_bytes = metrics.get_memory_used_bytes();
        let mem_total_bytes = metrics.get_memory_total_bytes();
        let self_cpu = metrics.get_self_cpu_usage();
        let self_mem_bytes = metrics.get_self_memory_bytes();

        let mem_used_gb = mem_used_bytes as f64 / 1024.0 / 1024.0 / 1024.0;
        let mem_total_gb = mem_total_bytes as f64 / 1024.0 / 1024.0 / 1024.0;
        let self_mem_mb = self_mem_bytes as f64 / 1024.0 / 1024.0;

        println!("\n📊 Telemetry Export at {}", timestamp);
        println!("─────────────────────────────────────────────────────────");
        println!("  Host: {}", hostname);
        println!("  CPU Utilization: {:.2}%", cpu_usage);
        println!("  CPU Time (accumulated): {:.3}s", cpu_time);
        println!("  Memory Utilization: {:.2}%", mem_usage_percent);
        println!("  Memory Usage: {:.2} GB / {:.2} GB", mem_used_gb, mem_total_gb);
        println!("  Memory Usage (bytes): {} / {}", mem_used_bytes, mem_total_bytes);
        println!("  [self] CPU: {:.2}%  Memory: {:.1} MB", self_cpu, self_mem_mb);
        match aranet4.get() {
            Some(r) => {
                let age = if r.age_secs < 60 {
                    format!("{}s ago", r.age_secs)
                } else {
                    format!("{}m ago", r.age_secs / 60)
                };
                println!("  [{}] CO2: {} ppm  Temp: {:.1}°C  Humidity: {}%  Pressure: {:.1} hPa  Battery: {}%  ({})",
                    r.device_name, r.co2_ppm, r.temperature_c, r.humidity_pct, r.pressure_hpa, r.battery_pct, age);
            }
            None => println!("  Aranet4: scanning..."),
        }
    }
}

/// Start HTTP server for Prometheus metrics endpoint
async fn start_http_server(registry: Arc<prometheus::Registry>) -> Result<()> {
    let registry_clone = registry.clone();
    
    let app = Router::new()
        .route("/metrics", get(move || metrics_handler(registry_clone.clone())))
        .route("/health", get(health_handler));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    
    println!("📡 Prometheus metrics endpoint: http://0.0.0.0:8080/metrics");
    println!("🏥 Health check endpoint: http://0.0.0.0:8080/health");
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt::init();

    println!("🚀 Starting Telemetry Service...");
    println!("📊 Collecting CPU and Memory metrics with OpenTelemetry");
    println!("📤 Publishing metrics to stdout every 5 seconds");
    println!("Press Ctrl+C to stop\n");

    // Initialize OpenTelemetry metrics with Prometheus exporter
    let (provider, registry) = init_metrics_with_prometheus()?;
    let registry = Arc::new(registry);
    let meter = global::meter("telemetry-service");

    // Create system metrics collector
    let metrics = Arc::new(SystemMetrics::new());

    // Give the system a moment to initialize CPU measurements
    time::sleep(Duration::from_secs(1)).await;

    // Register OpenTelemetry metrics
    register_metrics(&meter, metrics.clone())?;

    // Set up Aranet4 CO2 monitor
    let aranet4 = Arc::new(Aranet4Metrics::new());
    register_aranet4_metrics(&meter, aranet4.clone())?;

    // Start HTTP server for Prometheus endpoint
    let http_handle = {
        let registry = registry.clone();
        tokio::spawn(async move {
            if let Err(e) = start_http_server(registry).await {
                eprintln!("HTTP server error: {}", e);
            }
        })
    };

    // Start Aranet4 collection loop
    let aranet4_handle = tokio::spawn(aranet4_collection_loop(aranet4.clone()));

    // Start metrics logging loop
    let logging_handle = tokio::spawn(metrics_logging_loop(metrics, aranet4));

    // Wait for Ctrl+C
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("\n\n🛑 Shutting down gracefully...");
        }
        _ = http_handle => {
            println!("\n\n⚠️  HTTP server terminated unexpectedly");
        }
        _ = aranet4_handle => {
            println!("\n\n⚠️  Aranet4 collection loop terminated unexpectedly");
        }
        _ = logging_handle => {
            println!("\n\n⚠️  Logging loop terminated unexpectedly");
        }
    }

    provider.shutdown()?;
    
    Ok(())
}
