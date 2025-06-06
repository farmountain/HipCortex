mod memory_record;
mod memory_store;
mod memory_processor;
mod memory_query;
mod snapshot_manager;
mod memory_cli;

fn main() -> anyhow::Result<()> {
    memory_cli::run()
}
