/**
 * 日期工具
 */

import dayjs from "dayjs";

/**
 * 格式化日期
 */
export function formatDate(date: string | Date, format = "YYYY-MM-DD"): string {
  return dayjs(date).format(format);
}

/**
 * 格式化日期时间
 */
export function formatDateTime(date: string | Date): string {
  return dayjs(date).format("YYYY-MM-DD HH:mm:ss");
}

/**
 * 获取当前日期
 */
export function today(): string {
  return dayjs().format("YYYY-MM-DD");
}

/**
 * 获取当前年月
 */
export function currentYearMonth(): string {
  return dayjs().format("YYYY-MM");
}

/**
 * 解析年月
 */
export function parseYearMonth(yearMonth: string): { year: number; month: number } {
  const parts = yearMonth.split("-");
  const year = Number(parts[0] ?? 0);
  const month = Number(parts[1] ?? 0);
  return { year, month };
}
