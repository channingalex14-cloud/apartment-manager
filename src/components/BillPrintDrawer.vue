<script setup lang="ts">
import { computed } from 'vue'
import { formatMoneyInt, toYuanString } from '@/utils/money'
import type { BillDetailResponse } from '@/types/bill'
import type { RoomResponse } from '@/types/room'

const props = defineProps<{
  modelValue: boolean
  bill: BillDetailResponse | null
  configs: Record<string, string>
  room: RoomResponse | null
}>()

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
}>()

// 本地 bill 引用（安全访问）
const billRef = computed(() => props.bill)

// 水电用量
const waterUsage = computed(() => {
  const b = billRef.value
  if (!b) return 0
  return b.water_reading_current - b.water_reading_prev
})

const electricUsage = computed(() => {
  const b = billRef.value
  if (!b) return 0
  return b.electric_reading_current - b.electric_reading_prev
})

// 合计（四舍五入整数）
const totalAmount = computed(() => {
  const b = billRef.value
  if (!b) return '0'
  return formatMoneyInt(b.total_amount)
})

// 单价（后端存储的 key 为中文，值单位为分，如 水费单价=600 表示 6 元/吨）
const waterUnitPrice = computed(() => toYuanString(Number(props.configs['水费单价'] || 0)))
const electricUnitPrice = computed(() => toYuanString(Number(props.configs['电费单价'] || 0)))
const managementUnitPrice = computed(() => toYuanString(Number(props.configs['管理费单价'] || 0)))

// 温馨提示（去掉"转账记录请发截图登记"）
const tipsDisplay = computed(() => {
  const raw = props.configs['温馨提示'] || '请及时缴纳房租水电。逾期收取滞纳金。'
  const base = raw.replace(/转账记录请发截图登记[，,]?/g, '').replace(/^[,，。.\s]+|[,，。.\s]+$/g, '').trim() || '请及时缴纳房租水电。逾期收取滞纳金'
  return base.endsWith('。') ? base : base + '。'
})

// 收款二维码
const qrImageUrl = computed(() => {
  const filename = props.configs['收款二维码']
  if (!filename) return ''
  return `/bill-assets/${filename}`
})

// 账期字符串
const yearMonthStr = computed(() => {
  if (!props.bill?.year_month) return ''
  const [y, m] = props.bill.year_month.split('-')
  return `${y}年${parseInt(m || '1')}月`
})

function doPrint() {
  window.print()
}
</script>

<template>
  <el-dialog
    :model-value="modelValue"
    @update:model-value="emit('update:modelValue', $event)"
    :title="`${props.room?.room_number || ''} 收费通知单`"
    width="1100px"
    :close-on-click-modal="true"
    destroy-on-close
  >
    <div v-if="props.bill" class="bpd-print-area">
      <div class="bpd-card">

        <!-- 顶部标题 -->
        <div class="bpd-top">
          <div class="bpd-title-group">
            <div class="bpd-title">{{ props.configs['公寓名称'] || '新逸公寓' }}收费通知单</div>
            <div class="bpd-room-tag">
              <span>{{ props.room?.building || '' }}</span>
              <span class="bpd-div">|</span>
              <span>{{ props.bill.room_number }} 室</span>
              <span class="bpd-div">|</span>
              <span>{{ yearMonthStr }}</span>
            </div>
          </div>
        </div>

        <!-- 主体内容 -->
        <div class="bpd-main">
          <!-- 左侧费用表 -->
          <div class="bpd-table-area">
            <table class="bpd-table">
              <thead>
                <tr>
                  <th class="bpd-th-item">费用项目</th>
                  <th class="bpd-th-num">本月读数</th>
                  <th class="bpd-th-num">上月读数</th>
                  <th class="bpd-th-num">用量</th>
                  <th class="bpd-th-num">单价</th>
                  <th class="bpd-th-amt">应付金额</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td class="bpd-item">水费</td>
                  <td class="bpd-n">{{ props.bill.water_reading_current }}</td>
                  <td class="bpd-n">{{ props.bill.water_reading_prev }}</td>
                  <td class="bpd-n">{{ waterUsage }} 吨</td>
                  <td class="bpd-n">{{ waterUnitPrice }}/吨</td>
                  <td class="bpd-n bpd-amt">¥ {{ formatMoneyInt(props.bill.water_fee) }}</td>
                </tr>
                <tr>
                  <td class="bpd-item">电费</td>
                  <td class="bpd-n">{{ props.bill.electric_reading_current }}</td>
                  <td class="bpd-n">{{ props.bill.electric_reading_prev }}</td>
                  <td class="bpd-n">{{ electricUsage }} 度</td>
                  <td class="bpd-n">{{ electricUnitPrice }}/度</td>
                  <td class="bpd-n bpd-amt">¥ {{ formatMoneyInt(props.bill.electric_fee) }}</td>
                </tr>
                <tr>
                  <td class="bpd-item">管理费</td>
                  <td class="bpd-n">—</td>
                  <td class="bpd-n">—</td>
                  <td class="bpd-n">—</td>
                  <td class="bpd-n">{{ managementUnitPrice }}/月</td>
                  <td class="bpd-n bpd-amt">¥ {{ formatMoneyInt(props.bill.management_fee) }}</td>
                </tr>
                <tr>
                  <td class="bpd-item">房租</td>
                  <td class="bpd-n">—</td>
                  <td class="bpd-n">—</td>
                  <td class="bpd-n">—</td>
                  <td class="bpd-n">—</td>
                  <td class="bpd-n bpd-amt">¥ {{ formatMoneyInt(props.bill.rent_fee) }}</td>
                </tr>
                <tr>
                  <td class="bpd-item">物业费</td>
                  <td class="bpd-n">—</td>
                  <td class="bpd-n">—</td>
                  <td class="bpd-n">—</td>
                  <td class="bpd-n">—</td>
                  <td class="bpd-n bpd-amt">¥ {{ formatMoneyInt(props.bill.property_fee) }}</td>
                </tr>
                <tr>
                  <td class="bpd-item">维修费</td>
                  <td class="bpd-n">—</td>
                  <td class="bpd-n">—</td>
                  <td class="bpd-n">—</td>
                  <td class="bpd-n">—</td>
                  <td class="bpd-n bpd-amt">¥ {{ formatMoneyInt(props.bill.repair_fee) }}</td>
                </tr>
                <tr v-if="props.bill.misc_fee > 0">
                  <td class="bpd-item">{{ props.bill.misc_fee_remark || '杂费' }}</td>
                  <td class="bpd-n">—</td>
                  <td class="bpd-n">—</td>
                  <td class="bpd-n">—</td>
                  <td class="bpd-n">—</td>
                  <td class="bpd-n bpd-amt">¥ {{ formatMoneyInt(props.bill.misc_fee) }}</td>
                </tr>
                <!-- 合计行 -->
                <tr class="bpd-total-row">
                  <td class="bpd-item bpd-total-label-cell">合计</td>
                  <td class="bpd-n" colspan="4"></td>
                  <td class="bpd-n bpd-total-amt-cell">¥ {{ formatMoneyInt(props.bill.total_amount) }}</td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- 右侧二维码 -->
          <div v-if="qrImageUrl" class="bpd-qr-col">
            <div class="bpd-qr-label">转账记录请发截图登记，谢谢合作。</div>
            <div class="bpd-qr-box">
              <img :src="qrImageUrl" alt="收款二维码" />
            </div>
          </div>
        </div>

        <!-- 温馨提示在二维码下方 -->
        <div class="bpd-tips-bar">{{ tipsDisplay }}</div>

      </div>
    </div>

    <template #footer>
      <div class="bpd-footer">
        <el-button @click="emit('update:modelValue', false)">关闭</el-button>
        <el-button type="primary" @click="doPrint">🖨 打印</el-button>
      </div>
    </template>
  </el-dialog>
</template>

<style scoped lang="scss">
.bpd-print-area {
  display: flex;
  justify-content: center;
}

.bpd-card {
  background: #fff;
  border-radius: 10px;
  overflow: hidden;
  box-shadow: 0 6px 24px rgba(30, 60, 120, 0.10);
  width: 100%;
  max-width: 1050px;
}

// 顶部标题
.bpd-top {
  background: linear-gradient(135deg, #2C4A7C 0%, #3A5F9C 50%, #2C4A7C 100%);
  padding: 16px 24px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.bpd-title-group {
  text-align: center;
}

.bpd-title {
  font-size: 28px;
  font-weight: 900;
  color: #fff;
  letter-spacing: 8px;
}

.bpd-room-tag {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  margin-top: 4px;
  font-size: 18px;
  color: rgba(255, 255, 255, 0.85);
  font-weight: 600;
}

.bpd-div {
  color: rgba(255, 255, 255, 0.4);
}

// 主体
.bpd-main {
  display: flex;
  min-height: 300px;
}

// 费用表格
.bpd-table-area {
  flex: 1;
  padding: 14px 16px;
}

.bpd-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.bpd-table thead tr {
  background: linear-gradient(135deg, #2C4A7C, #3A5F9C);
}

.bpd-table th {
  color: #fff;
  font-weight: 700;
  padding: 8px 10px;
  text-align: center;
  font-size: 16px;
  letter-spacing: 0.5px;
}

.bpd-th-item {
  text-align: left !important;
  border-radius: 6px 0 0 6px;
}

.bpd-th-amt {
  text-align: right !important;
  border-radius: 0 6px 6px 0;
}

.bpd-table tbody tr {
  border-bottom: 1px solid #E8EFF8;
  transition: background 0.15s;
}

.bpd-table tbody tr:nth-child(even) {
  background: #F5F8FC;
}

.bpd-item {
  padding: 9px 10px;
  font-weight: 700;
  color: #2C4A7C;
  font-size: 16px;
}

.bpd-n {
  padding: 9px 10px;
  text-align: center;
  color: #555;
  font-family: "SF Mono", "Menlo", "Consolas", monospace;
  font-size: 16px;
}

.bpd-amt {
  text-align: left !important;
  font-weight: 900;
  color: #FF6B35;
  font-size: 16px;
  padding-left: 8px !important;
}

// 合计行
.bpd-total-row {
  border-top: 2px solid #2C4A7C;
  background: #F0F4F8;
}

.bpd-total-label-cell {
  font-size: 18px !important;
  font-weight: 900 !important;
  color: #2C4A7C !important;
  letter-spacing: 2px;
}

.bpd-total-amt-cell {
  font-size: 24px !important;
  font-weight: 900 !important;
  color: #FF6B35 !important;
  text-align: left !important;
}

// 温馨提示栏
.bpd-tips-bar {
  padding: 12px 20px;
  font-size: 24px;
  font-weight: 900;
  color: #FF4444;
  text-align: center;
  background: #2C4A7C;
}

// 右侧二维码
.bpd-qr-col {
  width: 340px;
  flex-shrink: 0;
  background: linear-gradient(180deg, #F5F8FC, #EEF2F7);
  border-left: 3px solid #DDE5F0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 20px 16px;
  gap: 12px;
}

.bpd-qr-label {
  font-size: 18px;
  font-weight: 800;
  color: #2C4A7C;
  letter-spacing: 3px;
  text-align: center;
}

.bpd-qr-box {
  width: 300px;
  height: 300px;
  background: #fff;
  border: 2px solid #DDE5F0;
  border-radius: 12px;
  padding: 10px;
  box-shadow: 0 4px 12px rgba(30, 60, 120, 0.08);

  img {
    width: 280px;
    height: 280px;
    object-fit: contain;
  }
}

// 底部操作栏
.bpd-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 12px;
}

// 打印样式
@media print {
  :deep(.el-dialog) {
    display: none !important;
  }
}
</style>
