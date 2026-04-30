<script setup lang="ts">
import { type RoomResponse, NO_CONTRACT_STATUSES } from '@/types/room'
import { formatMoneyInt } from '@/utils/money'

defineProps<{
  room: RoomResponse
  statusColor: string
  contractInfo: { daysLeft: number | null; expired: boolean; text: string; cssClass: string; icon: string } | null
}>()

const emit = defineEmits<{
  click: []
  print: []
}>()

function maskPhone(phone: string | null): string {
  if (!phone || phone.length < 7) return phone || '-'
  return phone.slice(0, 3) + '****' + phone.slice(-4)
}
</script>

<template>
  <div
    class="room-card"
    :class="`status-${room.status}`"
    @click="emit('click')"
  >
    <div class="card-header">
      <span class="room-number clickable" @click.stop="emit('print')">
        <span style="font-size: 17px">📍</span> {{ room.room_number }}
      </span>
      <span class="room-status-tag" :style="{ background: statusColor }">
        {{ room.status }}
      </span>
    </div>

    <div class="room-type">🏠 {{ room.room_type || '单间' }}</div>

    <div class="card-body">
      <div class="info-row tenant-name">
        <span class="info-icon">👤</span>
        <span class="info-value" v-if="room.tenant_name">{{ room.tenant_name }}</span>
        <span class="info-value vacant" v-else>{{ room.status }}</span>
      </div>
      <div class="info-row">
        <span class="info-icon">📞</span>
        <span class="info-value">{{ room.tenant_phone ? maskPhone(room.tenant_phone) : '-' }}</span>
      </div>
      <div class="info-row">
        <span class="info-icon">💰</span>
        <span class="info-value rent-value">{{ room.base_rent_fen > 0 ? formatMoneyInt(room.base_rent_fen) + '/月' : '-' }}</span>
      </div>
    </div>

    <div class="card-footer">
      <div class="contract-info">
        <span class="info-icon">📅</span>
        <span>{{ !NO_CONTRACT_STATUSES.has(room.status) && room.lease_end_date ? '合同至: ' + room.lease_end_date : '-' }}</span>
      </div>
      <div class="expiry-reminder" :class="contractInfo?.cssClass || ''">
        <span class="info-icon">{{ contractInfo?.icon }}</span>
        <span>{{ contractInfo?.text }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.room-card {
  position: relative;
  min-height: 140px;
  padding: 16px;
  border-radius: 12px;
  border: 2px solid var(--border-light);
  background: var(--bg-card);
  cursor: pointer;
  transition: all 0.25s ease;
  display: flex;
  flex-direction: column;
  gap: 8px;

  &:hover {
    transform: translateY(-3px);
    box-shadow: var(--shadow-card-hover);
  }

  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;

    .room-number {
      font-size: 22px;
      font-weight: bold;
      color: var(--text-primary);
    }

    .room-status-tag {
      font-size: 11px;
      color: #fff;
      padding: 2px 8px;
      border-radius: 4px;
      font-weight: 500;
      white-space: nowrap;
    }
  }

  .room-type {
    font-size: 12px;
    color: var(--text-secondary);
    background: transparent;
    padding: 2px 8px 2px 2px;
    border-radius: 4px;
    display: flex;
    align-items: center;
    gap: 6px;
    align-self: flex-start;
  }

  .card-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 5px;

    .info-row {
      display: flex;
      align-items: center;
      gap: 6px;
      font-size: 13px;
      color: var(--text-regular);

      .info-icon {
        font-size: 13px;
        flex-shrink: 0;
      }

      .info-value {
        color: var(--text-primary);

        &.vacant {
          color: var(--text-secondary);
          font-style: italic;
        }

        &.rent-value {
          font-weight: 600;
          color: var(--color-primary);
        }
      }

      &.tenant-name .info-value {
        font-size: 15px;
        font-weight: 600;
      }
    }
  }

  .card-footer {
    border-top: 1px dashed var(--border-light);
    padding-top: 8px;
    margin-top: auto;
    display: flex;
    flex-direction: column;
    gap: 3px;
    font-size: 12px;
    color: var(--text-secondary);

    .contract-info,
    .expiry-reminder {
      display: flex;
      align-items: center;
      gap: 5px;
    }

    .info-icon {
      font-size: 12px;
      flex-shrink: 0;
    }

    .expiry-reminder {
      font-weight: 500;

      &.safe { color: var(--color-success); }
      &.expiring { color: var(--color-warning); }
      &.expired { color: var(--color-danger); }
    }
  }

  &.status-在租 {
    border-color: var(--room-rented);
    background: linear-gradient(135deg, var(--room-rented-bg-start) 0%, var(--bg-card) 100%);
  }
  &.status-新租 {
    border-color: var(--room-new);
    background: linear-gradient(135deg, var(--room-new-bg-start) 0%, var(--bg-card) 100%);
  }
  &.status-空房 {
    border-color: var(--room-vacant);
    background: linear-gradient(135deg, var(--room-vacant-bg-start) 0%, var(--bg-card) 100%);
    opacity: 0.75;
  }
  &.status-员工 {
    border-color: var(--room-staff);
    background: linear-gradient(135deg, var(--room-staff-bg-start) 0%, var(--bg-card) 100%);
  }
  &.status-管理 {
    border-color: var(--room-manage);
    background: linear-gradient(135deg, var(--room-manage-bg-start) 0%, var(--bg-card) 100%);
  }
  &.status-违约 {
    border-color: #F56C6C;
    background: linear-gradient(135deg, rgba(245, 108, 108, 0.08) 0%, var(--bg-card) 100%);
  }
  &.status-待清洁 {
    border-color: #D463C8;
    background: linear-gradient(135deg, rgba(212, 99, 200, 0.08) 0%, var(--bg-card) 100%);
  }
  &.status-维修中 {
    border-color: #E6A23C;
    background: linear-gradient(135deg, rgba(230, 162, 60, 0.08) 0%, var(--bg-card) 100%);
  }
}
</style>
