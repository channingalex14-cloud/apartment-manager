<script setup lang="ts">
import { type RoomResponse, NO_CONTRACT_STATUSES } from '@/types/room'
import type { BillListItem } from '@/types/bill'
import type { MeterBillDetail } from '@/services/room.service'
import { formatMoneyInt } from '@/utils/money'

const props = defineProps<{
  room: RoomResponse
  recentBills: BillListItem[]
  meterDetail: MeterBillDetail | null
  statusColor: string
  statusBg: string
  statusTagType: 'primary' | 'success' | 'warning' | 'info' | 'danger'
  contractInfo: { daysLeft: number | null; expired: boolean; text: string; cssClass: string; icon: string } | null
  statusEditMode: boolean
  selectedStatus: string
  statusOptions: { status: string; label: string }[]
}>()

const emit = defineEmits<{
  'update:statusEditMode': [value: boolean]
  'update:selectedStatus': [value: string]
  'change-status': []
  'go-deposit': []
  'go-fee': []
  'open-print': []
}>()
</script>

<template>
  <el-drawer
    :model-value="true"
    :title="`${room.room_number} 详情`"
    size="400px"
    direction="rtl"
    :with-header="false"
    @close="emit('update:statusEditMode', false)"
  >
    <div class="drawer-hero" :style="{ '--room-status-bg': statusBg, '--room-status-color': statusColor }">
      <div class="hero-main">
        <span class="hero-room clickable" @click="emit('open-print')">📍 {{ room.room_number }}</span>
        <el-tag :type="statusTagType" size="large">{{ room.status }}</el-tag>
      </div>
      <div class="hero-sub">
        {{ room.room_type || '单间' }}<span v-if="room.floor"> · {{ room.floor }}层</span>
      </div>
    </div>

    <div class="fee-row">
      <div class="fee-card fee-deposit" :style="{ '--room-status-bg': statusBg }">
        <div class="fee-card-label">押金</div>
        <div class="fee-card-value">{{ formatMoneyInt(room.deposit_fen) }}</div>
      </div>
      <div class="fee-card fee-rent" :style="{ '--room-status-bg': statusBg }">
        <div class="fee-card-label">月租</div>
        <div class="fee-card-value">{{ formatMoneyInt(room.base_rent_fen) }}</div>
      </div>
      <div class="fee-card fee-property" :style="{ '--room-status-bg': statusBg }">
        <div class="fee-card-label">物业费</div>
        <div class="fee-card-value">{{ formatMoneyInt(room.property_fee_fen) }}</div>
      </div>
    </div>

    <div class="info-card" :style="{ '--room-status-color': statusColor, '--room-status-bg': statusBg }">
      <div class="tenant-row">
        <span class="tenant-icon">👤</span>
        <span class="tenant-name">{{ room.tenant_name || '无租客' }}</span>
      </div>
      <div class="tenant-phone-row" v-if="room.tenant_phone">
        <span class="phone-label">📞</span>
        <span class="phone-value">{{ room.tenant_phone }}</span>
      </div>
    </div>

    <div class="info-card" v-if="!NO_CONTRACT_STATUSES.has(room.status)" :style="{ '--room-status-color': statusColor, '--room-status-bg': statusBg }">
      <div class="contract-row">
        <span class="contract-label">开始日期</span>
        <span class="contract-value">{{ room.lease_start_date || '-' }}</span>
      </div>
      <div class="contract-row">
        <span class="contract-label">到期日期</span>
        <span class="contract-value">{{ room.lease_end_date || '-' }}</span>
      </div>
      <div class="contract-row">
        <span class="contract-reminder">
          {{ contractInfo?.icon }} {{ contractInfo?.text }}
        </span>
      </div>
    </div>

    <div class="info-card" v-if="meterDetail" :style="{ '--room-status-color': statusColor, '--room-status-bg': statusBg }">
      <div class="meter-row">
        <span class="meter-icon">💧</span>
        <span class="meter-item-label">本月</span>
        <span class="meter-item-value">{{ meterDetail.water_reading_current }}</span>
        <span class="meter-arrow">→</span>
        <span class="meter-item-label">上月</span>
        <span class="meter-item-value">{{ meterDetail.water_reading_prev }}</span>
        <span class="meter-usage">= {{ meterDetail.water_reading_current - meterDetail.water_reading_prev }}度</span>
      </div>
      <div class="meter-row">
        <span class="meter-icon">⚡</span>
        <span class="meter-item-label">本月</span>
        <span class="meter-item-value">{{ meterDetail.electric_reading_current }}</span>
        <span class="meter-arrow">→</span>
        <span class="meter-item-label">上月</span>
        <span class="meter-item-value">{{ meterDetail.electric_reading_prev }}</span>
        <span class="meter-usage">= {{ meterDetail.electric_reading_current - meterDetail.electric_reading_prev }}度</span>
      </div>
    </div>

    <div class="info-card" v-if="meterDetail" :style="{ '--room-status-color': statusColor, '--room-status-bg': statusBg }">
      <div class="month-fee-row">
        <span class="month-fee-item">
          💧水费<br><strong>{{ formatMoneyInt(meterDetail.water_fee) }}</strong>
        </span>
        <span class="month-fee-item">
          ⚡电费<br><strong>{{ formatMoneyInt(meterDetail.electric_fee) }}</strong>
        </span>
        <span class="month-fee-item">
          🔧管理费<br><strong>{{ formatMoneyInt(meterDetail.management_fee) }}</strong>
        </span>
      </div>
    </div>

    <div class="info-card" :style="{ '--room-status-color': statusColor, '--room-status-bg': statusBg }">
      <div class="bill-list">
        <div v-for="bill in recentBills" :key="bill.id" class="bill-row">
          <span class="bill-month">{{ bill.year_month }}</span>
          <span class="bill-total">合<strong>{{ formatMoneyInt(bill.total_amount) }}</strong></span>
          <span class="bill-status" :class="bill.status === '已支付' ? 'paid' : 'unpaid'">{{ bill.status }}</span>
        </div>
        <div v-if="recentBills.length === 0" class="empty-hint">暂无费用记录</div>
      </div>
    </div>

    <template #footer>
      <div class="drawer-footer">
        <el-button @click="emit('go-deposit')">编辑押金</el-button>
        <el-button type="primary" @click="emit('go-fee')">费用录入</el-button>
        <el-button v-if="!statusEditMode" type="warning" @click="emit('update:statusEditMode', true)">修改状态</el-button>
        <template v-else>
          <el-select :model-value="selectedStatus" @update:model-value="emit('update:selectedStatus', $event)" placeholder="选择状态" style="width: 120px">
            <el-option v-for="s in statusOptions" :key="s.status" :label="s.label" :value="s.status" />
          </el-select>
          <el-button type="warning" @click="emit('change-status')">确认</el-button>
          <el-button @click="emit('update:statusEditMode', false)">取消</el-button>
        </template>
      </div>
    </template>
  </el-drawer>
</template>

<style scoped lang="scss">
.info-card {
  background: var(--room-status-bg, var(--bg-hover));
  border-radius: 10px;
  padding: 12px 14px;
  margin-bottom: 12px;
}

.drawer-hero {
  border-left: none;
  padding: 0 16px;
  margin: -20px -20px 20px -20px;
  background: var(--room-status-bg, var(--bg-hover));
  border-radius: 0 8px 0 0;
  height: 112px;
  display: flex;
  flex-direction: column;
  justify-content: center;

  .hero-main {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 4px;
  }

  .hero-room {
    font-size: 26px;
    font-weight: 800;
    color: var(--text-primary);
    letter-spacing: 1px;
    &.clickable {
      cursor: pointer;
      &:hover {
        color: var(--el-color-primary);
      }
    }
  }

  .hero-sub {
    font-size: 13px;
    color: var(--text-secondary);
  }
}

.fee-row {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 10px;
  margin-bottom: 12px;

  .fee-card {
    border-radius: 10px;
    padding: 12px 8px;
    text-align: center;
    background: var(--room-status-bg, var(--bg-hover));

    .fee-card-label {
      font-size: 10px;
      color: var(--text-secondary);
      margin-bottom: 4px;
      text-transform: uppercase;
      letter-spacing: 0.5px;
    }

    .fee-card-value {
      font-size: 18px;
      font-weight: 700;
    }

    &.fee-deposit .fee-card-value { color: #409EFF; }
    &.fee-rent .fee-card-value { color: #67C23A; }
    &.fee-property .fee-card-value { color: #E6A23C; }
  }
}

.tenant-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;

  .tenant-icon { font-size: 18px; }
  .tenant-name { font-size: 16px; font-weight: 600; color: var(--text-primary); }
}

.tenant-phone-row {
  display: flex;
  align-items: center;
  gap: 8px;

  .phone-label { font-size: 14px; }
  .phone-value { font-size: 14px; color: var(--text-primary); }
}

.contract-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 5px;
  font-size: 13px;

  .contract-label { color: var(--text-secondary); }
  .contract-value { color: var(--text-primary); font-weight: 500; }
  .contract-reminder {
    font-size: 12px;
    font-weight: 500;
    &.safe { color: var(--color-success); }
    &.expiring { color: var(--color-warning); }
    &.expired { color: var(--color-danger); }
  }
}

.meter-row {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 6px;
  font-size: 13px;

  .meter-icon { font-size: 16px; flex-shrink: 0; }
  .meter-item-label { color: var(--text-secondary); font-size: 11px; min-width: 26px; }
  .meter-item-value { color: var(--text-primary); font-weight: 600; min-width: 30px; }
  .meter-arrow { color: var(--text-placeholder); font-size: 12px; }
  .meter-usage { color: var(--text-primary); font-weight: 600; }
}

.month-fee-row {
  display: flex;
  align-items: center;
  gap: 0;
  justify-content: space-around;

  .month-fee-item {
    font-size: 12px;
    color: var(--text-secondary);
    text-align: center;
    line-height: 1.4;

    strong { font-size: 17px; font-weight: 700; }
    &:nth-child(1) strong { color: #409EFF; }
    &:nth-child(2) strong { color: #67C23A; }
    &:nth-child(3) strong { color: #E6A23C; }
  }
}

.bill-list {
  display: flex;
  flex-direction: column;
  gap: 6px;

  .bill-row {
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(255, 255, 255, 0);
    border-radius: 8px;
    padding: 8px 12px;
    font-size: 13px;

    .bill-month { color: var(--text-primary); font-weight: 600; min-width: 52px; }
    .bill-total {
      flex: 1;
      color: var(--text-primary);
      strong { font-size: 15px; font-weight: 700; color: var(--color-primary); }
    }
    .bill-status {
      font-size: 11px;
      padding: 2px 8px;
      border-radius: 4px;
      font-weight: 500;
      &.paid { background: rgba(103, 194, 58, 0.1); color: var(--color-success); }
      &.unpaid { background: rgba(245, 108, 108, 0.1); color: var(--color-danger); }
    }
  }

  .empty-hint {
    font-size: 13px;
    color: var(--text-placeholder);
    text-align: center;
    padding: 12px;
  }
}

.drawer-footer {
  display: flex;
  gap: 10px;
  .el-button { flex: 1; }
}
</style>
