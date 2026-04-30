/**
 * 表单验证 Schema
 * 使用 lazySchema 避免循环依赖
 *
 * 引入 Claude Code 的 Zod 延迟构造模式：lazySchema
 * 保证 schema 在首次使用时才构造，避免模块初始化循环依赖
 *
 * @example
 * import { validateRoomForm, validateTenantForm } from '@/schemas'
 *
 * const result = validateRoomForm(data)
 * if (!result.success) {
 *   ElMessage.error(formatZodErrors(result.error))
 * }
 */

import { z } from 'zod'
import { lazySchema } from '@/utils/lazySchema'
import { formatZodError } from '@/utils/json'

// ---------------------------------------------------------------------------
// 基础类型 Schema（互相引用）
// ---------------------------------------------------------------------------

// 提前声明，用于 lazySchema 工厂
const RoomFormSchemaFactory = () =>
  z.object({
    room_number: z.string().min(1, '房号不能为空').max(20, '房号最长20字符'),
    building: z.string().optional().default(''),
    floor: z.number().int().min(0).max(999).optional(),
    room_type: z.string().optional().default('单间'),
    base_rent: z.number().int().min(0, '租金不能为负'),
    deposit: z.number().int().min(0, '押金不能为负'),
    property_fee: z.number().int().min(0).optional().default(0),
    status: z.enum(['在租', '新租', '空房', '员工', '管理', '违约', '待清洁', '维修中']),
    tenant_name: z.string().max(50).optional(),
    tenant_phone: z
      .string()
      .regex(/^1[3-9]\d{9}$/, '请输入有效的手机号')
      .optional()
      .or(z.literal('')),
    lease_start_date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/, '日期格式应为 YYYY-MM-DD').optional().or(z.literal('')),
    lease_end_date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/, '日期格式应为 YYYY-MM-DD').optional().or(z.literal('')),
  })

const TenantFormSchemaFactory = () =>
  z.object({
    name: z.string().min(1, '姓名不能为空').max(50),
    phone: z.string().regex(/^1[3-9]\d{9}$/, '请输入有效的手机号'),
    id_card: z.string().regex(/^[1-9]\d{5}(19|20)\d{2}(0[1-9]|1[0-2])(0[1-9]|[12]\d|3[01])\d{3}[\dXx]$/, '请输入有效的身份证号').optional().or(z.literal('')),
    emergency_contact: z.string().max(50).optional(),
    emergency_phone: z.string().regex(/^1[3-9]\d{9}$/, '请输入有效的手机号').optional().or(z.literal('')),
    move_in_date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/).optional(),
    move_out_date: z.string().regex(/^\d{4}-\d{2}-\d{2}$/).optional(),
    deposit_paid: z.number().int().min(0).optional().default(0),
    rent_paid: z.number().int().min(0).optional().default(0),
    remarks: z.string().max(500).optional(),
  })

const BillFormSchemaFactory = () =>
  z.object({
    room_id: z.number().int().positive('请选择房间'),
    year_month: z.string().regex(/^\d{4}-\d{2}$/, '账期格式应为 YYYY-MM'),
    water_reading_current: z.number().int().min(0, '读数不能为负'),
    water_reading_prev: z.number().int().min(0),
    electric_reading_current: z.number().int().min(0),
    electric_reading_prev: z.number().int().min(0),
    management_fee: z.number().int().min(0).optional().default(0),
    rent_fee: z.number().int().min(0).optional().default(0),
    property_fee: z.number().int().min(0).optional().default(0),
    repair_fee: z.number().int().min(0).optional().default(0),
    misc_fee: z.number().int().min(0).optional().default(0),
    misc_fee_remark: z.string().max(100).optional(),
    status: z.enum(['未支付', '部分支付', '已支付', '已作废']).optional().default('未支付'),
  })

// ---------------------------------------------------------------------------
// 导出 lazy schema（延迟构造）
// ---------------------------------------------------------------------------

export const RoomFormSchema = lazySchema(RoomFormSchemaFactory)
export const TenantFormSchema = lazySchema(TenantFormSchemaFactory)
export const BillFormSchema = lazySchema(BillFormSchemaFactory)

// ---------------------------------------------------------------------------
// 验证函数
// ---------------------------------------------------------------------------

export type RoomFormData = z.infer<ReturnType<typeof RoomFormSchema>>
export type TenantFormData = z.infer<ReturnType<typeof TenantFormSchema>>
export type BillFormData = z.infer<ReturnType<typeof BillFormSchema>>

/**
 * 验证房间表单数据
 */
export function validateRoomForm(data: unknown): {
  success: boolean
  data?: RoomFormData
  errors?: string[]
} {
  const result = RoomFormSchema().safeParse(data)
  if (result.success) return { success: true, data: result.data }
  return { success: false, errors: formatZodError(result.error).split('\n') }
}

/**
 * 验证租客表单数据
 */
export function validateTenantForm(data: unknown): {
  success: boolean
  data?: TenantFormData
  errors?: string[]
} {
  const result = TenantFormSchema().safeParse(data)
  if (result.success) return { success: true, data: result.data }
  return { success: false, errors: formatZodError(result.error).split('\n') }
}

/**
 * 验证账单表单数据
 */
export function validateBillForm(data: unknown): {
  success: boolean
  data?: BillFormData
  errors?: string[]
} {
  const result = BillFormSchema().safeParse(data)
  if (result.success) return { success: true, data: result.data }
  return { success: false, errors: formatZodError(result.error).split('\n') }
}
