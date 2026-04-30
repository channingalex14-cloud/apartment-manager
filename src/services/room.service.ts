/**
 * 房间 API 服务
 */

import { callCommand } from "./api";
import type { RoomResponse } from "@/types/room";

/** 房间水电账单详情 */
export interface MeterBillDetail {
  year_month: string;
  /** 上期（本月之前）水表读数 */
  water_reading_prev: number;
  /** 本期水表读数 */
  water_reading_current: number;
  /** 上期（本月之前）电表读数 */
  electric_reading_prev: number;
  /** 本期电表读数 */
  electric_reading_current: number;
  water_fee: number;
  electric_fee: number;
  management_fee: number;
}

export interface UpdateRoomRequest {
  version: number;  // 乐观锁版本号
  base_rent?: number;
  property_fee?: number;
  water_meter_current?: number;
  electric_meter_current?: number;
  room_type?: string;
}

export const roomService = {
  /** 列出所有房间 */
  async list(): Promise<RoomResponse[]> {
    return callCommand<RoomResponse[]>("list_rooms");
  },

  /** 获取房间详情 */
  async get(id: number): Promise<RoomResponse | null> {
    return callCommand<RoomResponse | null>("get_room", { id });
  },

  /** 更新房间信息（乐观锁保护） */
  async update(id: number, data: UpdateRoomRequest): Promise<boolean> {
    return callCommand<boolean>("update_room", {
      id,
      version: data.version,
      base_rent: data.base_rent,
      property_fee: data.property_fee,
      water_meter_current: data.water_meter_current,
      electric_meter_current: data.electric_meter_current,
      room_type: data.room_type,
    });
  },

  /** 更新房间状态（手动修改特殊状态） */
  async updateStatus(id: number, status: string, operator?: string, notes?: string): Promise<boolean> {
    return callCommand<boolean>("update_room_status", { id, status, operator, notes });
  },

  /** 获取房间最新水电账单详情 */
  async getMeterDetail(roomId: number): Promise<MeterBillDetail | null> {
    return callCommand<MeterBillDetail | null>("get_room_meter_detail", { room_id: roomId });
  },
};
