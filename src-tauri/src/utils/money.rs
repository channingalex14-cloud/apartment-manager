//! 金额处理工具
//!
//! 所有金额单位统一使用「分」(CENT)

/// 分与元的换算倍率
pub const CENT: i64 = 100;

/// 元转分
///
/// # Example
/// ```
/// assert_eq!(to_cent(19.90), 1990);
/// assert_eq!(to_cent(2000.0), 200000);
/// ```
#[inline]
pub fn to_cent(yuan: f64) -> i64 {
    (yuan * CENT as f64).round() as i64
}

/// 分转元
///
/// # Example
/// ```
/// assert_eq!(crate::utils::money::to_yuan(1990), 19.90);
/// assert_eq!(crate::utils::money::to_yuan(200000), 2000.0);
/// ```
#[inline]
pub fn to_yuan(fen: i64) -> f64 {
    fen as f64 / CENT as f64
}

/// 格式化金额（分 -> 字符串元）
///
/// # Example
/// ```
/// assert_eq!(crate::utils::money::format_money(1990), "19.90");
/// assert_eq!(crate::utils::money::format_money(200000), "2000.00");
/// ```
#[inline]
pub fn format_money(fen: i64) -> String {
    format!("{:.2}", to_yuan(fen))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_cent() {
        assert_eq!(to_cent(19.90), 1990);
        assert_eq!(to_cent(2000.0), 200000);
        assert_eq!(to_cent(0.01), 1);
        assert_eq!(to_cent(0.005), 1); // 四舍五入
    }

    #[test]
    fn test_to_yuan() {
        assert_eq!(to_yuan(1990), 19.90);
        assert_eq!(to_yuan(200000), 2000.0);
        assert_eq!(to_yuan(1), 0.01);
    }

    #[test]
    fn test_format_money() {
        assert_eq!(format_money(1990), "19.90");
        assert_eq!(format_money(200000), "2000.00");
        assert_eq!(format_money(1), "0.01");
    }
}
