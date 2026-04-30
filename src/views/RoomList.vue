<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { useRoomStore } from '@/stores/room'
import { useUIStore } from '@/stores/ui'
import BillPrintDrawer from '@/components/BillPrintDrawer.vue'
import RoomCard from '@/components/room/RoomCard.vue'
import RoomDetailDrawer from '@/components/room/RoomDetailDrawer.vue'
import { billService } from '@/services/bill.service'
import { configService } from '@/services/config.service'
import { roomService, type MeterBillDetail } from '@/services/room.service'
import { type RoomResponse, NO_CONTRACT_STATUSES } from '@/types/room'
import type { BillListItem } from '@/types/bill'

const router = useRouter()
const roomStore = useRoomStore()
const uiStore = useUIStore()
const drawerVisible = ref(false)
const selectedRoom = ref<RoomResponse | null>(null)
const recentBills = ref<BillListItem[]>([])
const meterDetail = ref<MeterBillDetail | null>(null)
const statusEditMode = ref(false)
const selectedStatus = ref('')

const printDrawerVisible = ref(false)
const printBill = ref<Awaited<ReturnType<typeof billService.getBillDetail>> | null>(null)
const printConfigs = ref<Awaited<ReturnType<typeof configService.getAll>> | null>(null)
const printRoom = ref<RoomResponse | null>(null)

const printConfigsFormatted = computed<Record<string, string>>(() => {
  if (!printConfigs.value) return {}
  if (Array.isArray(printConfigs.value)) {
    const map: Record<string, string> = {}
    for (const c of printConfigs.value) {
      if (c.config_key) {
        map[c.config_key] = c.config_value ?? ''
      }
    }
    return map
  }
  return printConfigs.value as Record<string, string>
})

onMounted(() => {
  roomStore.fetchRooms()
})

const contractInfoCache = new Map<number, ReturnType<typeof getContractInfoRaw>>()

watch(() => roomStore.rooms, () => {
  contractInfoCache.clear()
})

const statusConfig = [
  { status: '在租', label: '在租', color: 'var(--room-rented)', bgStart: 'var(--room-rented-bg-start)' },
  { status: '新租', label: '新租', color: 'var(--room-new)', bgStart: 'var(--room-new-bg-start)' },
  { status: '空房', label: '空房', color: 'var(--room-vacant)', bgStart: 'var(--room-vacant-bg-start)' },
  { status: '员工', label: '员工', color: 'var(--room-staff)', bgStart: 'var(--room-staff-bg-start)' },
  { status: '管理', label: '管理', color: 'var(--room-manage)', bgStart: 'var(--room-manage-bg-start)' },
  { status: '违约', label: '违约', color: '#F56C6C', bgStart: 'rgba(245, 108, 108, 0.08)' },
  { status: '待清洁', label: '待清洁', color: '#D463C8', bgStart: 'rgba(212, 99, 200, 0.08)' },
  { status: '维修中', label: '维修中', color: '#E6A23C', bgStart: 'rgba(230, 162, 60, 0.08)' },
]

const filteredRooms = computed(() => {
  let result = roomStore.rooms
  if (uiStore.globalSearchKeyword) {
    const kw = uiStore.globalSearchKeyword.toLowerCase()
    result = result.filter(
      (r) =>
        r.room_number.toLowerCase() === kw ||
        r.tenant_name?.toLowerCase() === kw,
    )
  }
  return result
})

function getCountByStatus(status: string): number {
  return roomStore.rooms.filter((r) => r.status === status).length
}

function getStatusColor(room: RoomResponse): string {
  const config = statusConfig.find((c) => c.status === room.status)
  return config?.color || 'var(--room-vacant)'
}

function getStatusBg(room: RoomResponse): string {
  const config = statusConfig.find((c) => c.status === room.status)
  return config?.bgStart || 'var(--bg-hover)'
}

function getStatusTagType(status: string): 'primary' | 'success' | 'warning' | 'info' | 'danger' {
  const map: Record<string, 'primary' | 'success' | 'warning' | 'info' | 'danger'> = {
    '在租': 'success',
    '新租': 'warning',
    '空房': 'warning',
    '员工': 'success',
    '管理': 'success',
    '违约': 'danger',
    '待清洁': 'danger',
    '维修中': 'warning',
  }
  return map[status] ?? 'info'
}

function getContractInfo(room: RoomResponse): ReturnType<typeof getContractInfoRaw> | null {
  if (!contractInfoCache.has(room.id)) {
    contractInfoCache.set(room.id, getContractInfoRaw(room))
  }
  return contractInfoCache.get(room.id) ?? null
}

function getContractInfoRaw(room: RoomResponse): { daysLeft: number | null; expired: boolean; text: string; cssClass: string; icon: string } | null {
  if (NO_CONTRACT_STATUSES.has(room.status) || !room.lease_end_date) {
    return { daysLeft: null, expired: false, text: '-', cssClass: '', icon: '➖' }
  }
  const end = new Date(room.lease_end_date)
  if (isNaN(end.getTime())) {
    return { daysLeft: null, expired: false, text: '合同日期无效', cssClass: 'expired', icon: '⚠️' }
  }
  const today = new Date()
  today.setHours(0, 0, 0, 0)
  end.setHours(0, 0, 0, 0)
  const diffMs = end.getTime() - today.getTime()
  const daysLeft = Math.floor(diffMs / (1000 * 60 * 60 * 24))

  if (daysLeft < 0) {
    return { daysLeft, expired: true, text: `已到期 ${Math.abs(daysLeft)} 天`, cssClass: 'expired', icon: '🟠' }
  } else if (daysLeft <= 30) {
    return { daysLeft, expired: false, text: `还有 ${daysLeft} 天到期`, cssClass: 'expiring', icon: '🔔' }
  } else {
    return { daysLeft, expired: false, text: `还有 ${daysLeft} 天到期`, cssClass: 'safe', icon: '✅' }
  }
}

async function openRoomDetail(room: RoomResponse) {
  selectedRoom.value = room
  recentBills.value = []
  meterDetail.value = null
  statusEditMode.value = false
  selectedStatus.value = room.status
  if (room.id) {
    try {
      const [billRes, meterRes] = await Promise.all([
        billService.queryBills({ roomId: room.id, page: 1, pageSize: 6 }),
        roomService.getMeterDetail(room.id),
      ])
      recentBills.value = billRes.bills
      meterDetail.value = meterRes
    } catch (e) {
      console.error('查询详情失败:', e)
    }
  }
  drawerVisible.value = true
}

async function changeRoomStatus() {
  if (!selectedRoom.value || !selectedStatus.value) return
  const operator = '管理员'
  try {
    await roomStore.updateRoomStatus(selectedRoom.value.id, selectedStatus.value, operator)
    const updatedRoom = roomStore.getRoomById(selectedRoom.value.id)
    if (updatedRoom) {
      selectedRoom.value = updatedRoom
    }
    drawerVisible.value = false
  } catch (e) {
    ElMessage.error('修改状态失败: ' + (e instanceof Error ? e.message : String(e)))
  }
}

function goToDeposit() {
  drawerVisible.value = false
  router.push('/deposits')
}

function goToFee() {
  drawerVisible.value = false
  router.push('/bills')
}

const printLoading = ref(false)

async function openPrintDrawer(room: RoomResponse) {
  if (!room.id) {
    ElMessage.warning('房间信息无效')
    return
  }
  if (printLoading.value) return
  printLoading.value = true
  try {
    const res = await billService.queryBills({ roomId: room.id, page: 1, pageSize: 1 })
    if (res.bills.length === 0) {
      ElMessage.warning('该房间暂无账单')
      return
    }
    const firstBill = res.bills[0]!
    const [billData, configData] = await Promise.all([
      billService.getBillDetail(firstBill.id),
      configService.getAll(),
    ])
    printBill.value = billData
    const configMap: Record<string, string> = {}
    for (const c of configData) {
      if (c.config_key && c.config_value != null) {
        configMap[c.config_key] = c.config_value
      }
    }
    printConfigs.value = configMap as any
    printRoom.value = room
    printDrawerVisible.value = true
  } catch {
    ElMessage.error('加载账单失败')
  } finally {
    printLoading.value = false
  }
}

watch(printDrawerVisible, (isVisible) => {
  if (!isVisible) {
    printBill.value = null
    printConfigs.value = null
    printRoom.value = null
  }
})
</script>

<template>
  <div class="room-status-page">
    <div class="toolbar">
      <div class="legend">
        <span class="legend-item" v-for="item in statusConfig" :key="item.status">
          <span class="dot" :style="{ background: item.color }"></span>
          {{ item.label }}
        </span>
      </div>
    </div>

    <div class="summary-bar">
      <div class="summary-item" v-for="item in statusConfig" :key="item.status">
        <span class="count" :style="{ color: item.color }">{{ getCountByStatus(item.status) }}</span>
        <span class="label">{{ item.label }}</span>
      </div>
      <div class="summary-item total">
        <span class="count">{{ roomStore.rooms.length }}</span>
        <span class="label">总计</span>
      </div>
    </div>

    <div class="room-grid" v-loading="roomStore.loading">
      <RoomCard
        v-for="room in filteredRooms"
        :key="room.id"
        :room="room"
        :status-color="getStatusColor(room)"
        :contract-info="getContractInfo(room)"
        @click="openRoomDetail(room)"
        @print="openPrintDrawer(room)"
      />
    </div>

    <el-empty v-if="!roomStore.loading && roomStore.rooms.length === 0" description="暂无房间数据" />

    <RoomDetailDrawer
      v-if="drawerVisible && selectedRoom"
      :room="selectedRoom"
      :recent-bills="recentBills"
      :meter-detail="meterDetail"
      :status-color="getStatusColor(selectedRoom)"
      :status-bg="getStatusBg(selectedRoom)"
      :status-tag-type="getStatusTagType(selectedRoom.status)"
      :contract-info="getContractInfo(selectedRoom)"
      :status-edit-mode="statusEditMode"
      :selected-status="selectedStatus"
      :status-options="statusConfig"
      @update:status-edit-mode="statusEditMode = $event"
      @update:selected-status="selectedStatus = $event"
      @change-status="changeRoomStatus"
      @go-deposit="goToDeposit"
      @go-fee="goToFee"
      @open-print="openPrintDrawer(selectedRoom!)"
      @close="drawerVisible = false"
    />

    <BillPrintDrawer
      v-model="printDrawerVisible"
      :bill="printBill"
      :configs="printConfigsFormatted"
      :room="printRoom"
    />
  </div>
</template>

<style scoped lang="scss">
.room-status-page {
  padding: 16px;
}

.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;

  .legend {
    display: flex;
    gap: 16px;
    .legend-item {
      display: flex;
      align-items: center;
      gap: 6px;
      font-size: 13px;
      .dot {
        width: 12px;
        height: 12px;
        border-radius: 3px;
      }
    }
  }
}

.summary-bar {
  display: flex;
  gap: 24px;
  margin-bottom: 20px;
  padding: 12px 20px;
  background: var(--bg-hover);
  border-radius: 8px;

  .summary-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    .count {
      font-size: 22px;
      font-weight: bold;
    }
    .label {
      font-size: 12px;
      color: var(--text-secondary);
    }
  }
}

.room-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 14px;
  align-items: stretch;
}
</style>
