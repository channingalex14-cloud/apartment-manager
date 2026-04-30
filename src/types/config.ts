/**
 * 系统配置类型定义
 */

export interface SystemConfig {
  id: number;
  config_key: string;
  config_value: string | null;
  config_type: string | null;
  description: string | null;
  is_active: boolean;
}
