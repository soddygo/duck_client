mod types;
mod parser;
mod generator;
mod differ;

#[cfg(test)]
mod tests;

// 重新导出公共接口
pub use generator::generate_schema_diff; 