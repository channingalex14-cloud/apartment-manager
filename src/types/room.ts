/**
 * 房间类型定义
 */

export type RoomStatus =
  | "空房"
  | "在租"
  | "新租"
  | "员工"
  | "管理"
  | "违约"
  | "维修中"
  | "待清洁";

export interface RoomResponse {
  id: number;
  room_number: string;
  floor: number | null;
  building: string;
  room_type: string;
  base_rent_fen: number;
  property_fee_fen: number;
  deposit_fen: number;
  status: RoomStatus;
  water_meter_current: number;
  electric_meter_current: number;
  tenant_name: string | null;
  tenant_phone: string | null;
  lease_id: number | null;
  lease_start_date: string | null;
  lease_end_date: string | null;
  version: number;  // 乐观锁版本号
}

/** 不需要显示合同到期信息的状态集合 */
export const NO_CONTRACT_STATUSES: ReadonlySet<RoomStatus> = new Set([
  '空房', '员工', '管理', '违约', '待清洁', '维修中',
] as const)
