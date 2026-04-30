/**
 * 账单收款二维码图片资产
 *
 * 使用 import.meta.glob 静态扫描 public/bill-assets/ 目录，
 * 构建时自动收集所有可用图片，前端可选择使用哪张。
 */

// 扫描 public/bill-assets/ 下所有图片（作为 URL）
// 从 src/utils/ 到 public/ 需要 ../../
const imageModules = import.meta.glob<string>(
  "../../public/bill-assets/*",
  { eager: true, query: "?url", import: "default" }
);

// 提取文件名列表（按文件名排序）
export const availableQRImages: string[] = Object.keys(imageModules)
  .map((path) => {
    // path 格式: "/public/bill-assets/xxx.png" → "xxx.png"
    const parts = path.split("/");
    return parts[parts.length - 1] ?? "";
  })
  .filter(Boolean)
  .sort();

/**
 * 根据文件名获取图片的完整 URL
 */
export function getQRImageUrl(filename: string): string {
  if (!filename) return "";
  return `/bill-assets/${filename}`;
}
