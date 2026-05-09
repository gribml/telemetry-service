use sysinfo::{System, RefreshKind, CpuRefreshKind, MemoryRefreshKind};

#[test]
fn test_system_metrics_collection() {
    let mut system = System::new_with_specifics(
        RefreshKind::new()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything()),
    );
    
    // Give CPU time to initialize
    std::thread::sleep(std::time::Duration::from_millis(200));
    
    system.refresh_cpu();
    system.refresh_memory();
    
    // Test CPU metrics
    let cpu_usage = system.global_cpu_info().cpu_usage();
    assert!(cpu_usage >= 0.0, "CPU usage should be non-negative");
    assert!(cpu_usage <= 100.0 * system.cpus().len() as f32, "CPU usage should be reasonable");
    
    // Test memory metrics
    let total_memory = system.total_memory();
    let used_memory = system.used_memory();
    
    assert!(total_memory > 0, "Total memory should be positive");
    assert!(used_memory > 0, "Used memory should be positive");
    assert!(used_memory <= total_memory, "Used memory should not exceed total memory");
    
    // Test memory percentage calculation
    let memory_percent = (used_memory as f64 / total_memory as f64) * 100.0;
    assert!(memory_percent >= 0.0 && memory_percent <= 100.0, "Memory percentage should be between 0 and 100");
}

#[test]
fn test_memory_unit_conversions() {
    let bytes = 17179869184u64; // 16 GB
    let gb = bytes as f64 / 1024.0 / 1024.0 / 1024.0;
    
    assert!((gb - 16.0).abs() < 0.01, "16 GB conversion should be accurate");
}

#[test]
fn test_hostname_retrieval() {
    let hostname = hostname::get();
    assert!(hostname.is_ok(), "Should be able to get hostname");
    
    let hostname_str = hostname.unwrap().to_string_lossy().to_string();
    assert!(!hostname_str.is_empty(), "Hostname should not be empty");
}

#[cfg(test)]
mod opentelemetry_tests {
    use opentelemetry::{global, KeyValue};
    use opentelemetry_sdk::{metrics::SdkMeterProvider, Resource};

    #[test]
    fn test_meter_provider_initialization() {
        let provider = SdkMeterProvider::builder()
            .with_resource(Resource::new(vec![
                KeyValue::new("service.name", "test-service"),
                KeyValue::new("service.version", "0.0.1"),
            ]))
            .build();

        global::set_meter_provider(provider.clone());
        
        let _meter = global::meter("test-meter");
        // Meter created successfully
        
        // Cleanup
        let _ = provider.shutdown();
    }

    #[test]
    fn test_observable_gauge_creation() {
        let provider = SdkMeterProvider::builder()
            .with_resource(Resource::new(vec![
                KeyValue::new("service.name", "test-service"),
            ]))
            .build();

        global::set_meter_provider(provider.clone());
        let meter = global::meter("test-meter");
        
        // Create a simple observable gauge
        let _gauge = meter
            .f64_observable_gauge("test.metric")
            .with_description("A test metric")
            .with_callback(|observer| {
                observer.observe(42.0, &[KeyValue::new("test", "value")]);
            })
            .init();
        
        // If we got here without panicking, the gauge was created successfully
        let _ = provider.shutdown();
    }
}
