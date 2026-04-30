/**
 * 抄表 API 服务
 */

import { callCommand } from "./api";

export interface MeterReadingRequest {
  room_id: number;
  year: number;
  month: number;
  water_reading: number;
  electric_reading: number;
  reading_date: string;
  operator?: string;
  is_replacement: boolean;
}

export interface MeterReadingResponse {
  success: boolean;
  message: string;
  id: number | null;
}

export interface BatchMeterReadingRequest {
  readings: MeterReadingRequest[];
}

export const meterReadingService = {
  /** 单条录入抄表读数 */
  async record(req: MeterReadingRequest): Promise<MeterReadingResponse> {
    return callCommand<MeterReadingResponse>("record_meter_reading", { req });
  },

  /** 批量录入抄表读数 */
  async batchRecord(req: BatchMeterReadingRequest): Promise<MeterReadingResponse[]> {
    return callCommand<MeterReadingResponse[]>("batch_record_meter_readings", { req });
  },
};