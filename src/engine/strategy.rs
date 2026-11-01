use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyStrategy {
    Auto,
    Parallel,
    Reflink,
    CopyFile2,
    DirectIo,
    Buffered,
}

impl FromStr for CopyStrategy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(CopyStrategy::Auto),
            "parallel" | "streams" | "multi" | "multipart" | "turbo" => Ok(CopyStrategy::Parallel),
            "reflink" | "cow" | "clone" => Ok(CopyStrategy::Reflink),
            "copyfile2" | "win" | "kernel" => Ok(CopyStrategy::CopyFile2),
            "direct" | "directio" | "unbuffered" | "dma" => Ok(CopyStrategy::DirectIo),
            "buffered" | "pipe" => Ok(CopyStrategy::Buffered),
            _ => Err(format!(
                "Unknown engine strategy '{}'. Valid values: auto, parallel, reflink, copyfile2, direct, buffered",
                s
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub strategy: CopyStrategy,
    pub buffer_size: usize,
    pub no_buffering_threshold: u64,
    pub preallocate: bool,
    pub preserve_attributes: bool,
    pub parallel_streams: usize,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            strategy: CopyStrategy::Auto,
            // 4 MB default transfer buffer: ideal for NVMe deep queue depth & sustained line rate
            buffer_size: 4 * 1024 * 1024,
            // Files larger than 8 MB automatically bypass OS cache to eliminate RAM thrashing
            no_buffering_threshold: 8 * 1024 * 1024,
            preallocate: true,
            preserve_attributes: true,
            parallel_streams: num_cpus::get() * 2,
        }
    }
}
