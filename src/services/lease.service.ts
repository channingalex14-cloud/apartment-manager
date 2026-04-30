/**
 * 合同 API 服务
 */

import { callCommand } from "./api";
import type { Lease, LeaseResponse } from "@/types/lease";

export interface CheckInRequest {
  room_id: number;
  tenant_id: number;
  lease_id: number;
  move_in_date: string;
  operator?: string;
}

export interface CheckOutRequest {
  lease_id: number;
  room_id: number;
  move_out_date: string;
  reason: string;
  operator?: string;
}

export interface CreateLeaseRequest {
  room_id: number;
  tenant_id: number;
  start_date: string;
  monthly_rent: number;
  property_fee: number;
  deposit: number;
  contract_number?: string;
  end_date?: string;
}

export const leaseService = {
  /** 列出所有合同 */
  async list(): Promise<Lease[]> {
    return callCommand<Lease[]>("list_leases");
  },

  /** 获取合同详情 */
  async get(id: number): Promise<Lease | null> {
    return callCommand<Lease | null>("get_lease", { id });
  },

  /** 创建合同 */
  async create(req: CreateLeaseRequest): Promise<number> {
    return callCommand<number>("create_lease", {
      room_id: req.room_id,
      tenant_id: req.tenant_id,
      start_date: req.start_date,
      monthly_rent: req.monthly_rent,
      property_fee: req.property_fee,
      deposit: req.deposit,
      contract_number: req.contract_number,
      end_date: req.end_date,
    });
  },

  /** 激活合同 */
  async activate(id: number): Promise<boolean> {
    return callCommand<boolean>("activate_lease", { id });
  },

  /** 入住 */
  async checkIn(req: CheckInRequest): Promise<LeaseResponse> {
    return callCommand<LeaseResponse>("check_in", { req });
  },

  /** 退房 */
  async checkOut(req: CheckOutRequest): Promise<LeaseResponse> {
    return callCommand<LeaseResponse>("check_out", { req });
  },

  /** 违约标记 */
  async markViolation(leaseId: number): Promise<LeaseResponse> {
    return callCommand<LeaseResponse>("mark_violation", { lease_id: leaseId });
  },

  /** 违约恢复 */
  async recoverFromViolation(leaseId: number): Promise<LeaseResponse> {
    return callCommand<LeaseResponse>("recover_from_violation", { lease_id: leaseId });
  },
};
