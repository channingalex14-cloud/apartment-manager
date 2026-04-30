/**
 * 金额处理工具
 *
 * 重要：所有金额在前后端之间以「分」为单位传输
 * 前端只负责展示和输入转换
 */

const CENT = 100;

/**
 * 元转分
 *
 * 避免浮点陷阱，使用字符串解析
 */
export function toCent(yuan: number | string): number {
  const valueStr = typeof yuan === "string" ? yuan : String(yuan);

  const isNegative = valueStr.startsWith("-");
  const absValueStr = isNegative ? valueStr.slice(1) : valueStr;

  const parts = absValueStr.split(".");
  const intPart = parseInt(parts[0] || "0", 10) * CENT;
  const decPart = parts[1] ? parts[1].padEnd(2, "0").slice(0, 2) : "00";
  const result = intPart + parseInt(decPart, 10);

  return isNegative ? -result : result;
}

/**
 * 分转元（保留2位小数）
 */
export function toYuanString(fen: number): string {
  return (fen / CENT).toFixed(2);
}

/**
 * 分转元（数字）
 */
export function toYuanNumber(fen: number): number {
  return Math.round((fen / CENT) * 100) / 100;
}

/**
 * 格式化金额显示（输入为分）
 */
export function formatMoney(fen: number, symbol = "¥"): string {
  return `${symbol}${new Intl.NumberFormat("zh-CN", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(fen / CENT)}`;
}

/**
 * 格式化金额显示（输入为分，四舍五入取整）
 */
export function formatMoneyInt(fen: number, symbol = "¥"): string {
  return `${symbol}${Math.round(fen / CENT)}`;
}
