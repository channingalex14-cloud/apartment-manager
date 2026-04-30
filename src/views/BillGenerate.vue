<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { ElMessage } from "element-plus";
import { useRoomStore } from "@/stores/room";
import { billService, type GenerateBillsRequest } from "@/services/bill.service";
import { formatMoney } from "@/utils/money";

const roomStore = useRoomStore();

const loading = ref(false);
const generating = ref(false);
const yearMonth = ref("");
const selectedRoomIds = ref<number[]>([]);

// Meter readings for each room
interface MeterReading {
  room_id: number;
  water_reading: number;
  electric_reading: number;
}

const meterReadings = ref<MeterReading[]>([]);

// Get current year-month in YYYY-MM format
function getCurrentYearMonth(): string {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  return `${year}-${month}`;
}

// Initialize year-month to current month
yearMonth.value = getCurrentYearMonth();

// Rentable rooms (空房 or 在租)
const rentableRooms = computed(() => {
  return roomStore.rooms.filter((r) => r.status === "空房" || r.status === "在租");
});

onMounted(async () => {
  loading.value = true;
  try {
    await roomStore.fetchRooms();
    initMeterReadings();
    // Select all by default
    selectedRoomIds.value = rentableRooms.value.map((r) => r.id);
  } finally {
    loading.value = false;
  }
});

function initMeterReadings() {
  meterReadings.value = rentableRooms.value.map((room) => ({
    room_id: room.id,
    water_reading: room.water_meter_current || 0,
    electric_reading: room.electric_meter_current || 0,
  }));
}

function getMeterReading(roomId: number): MeterReading | undefined {
  return meterReadings.value.find((m) => m.room_id === roomId);
}

async function handleGenerate() {
  if (!yearMonth.value) {
    ElMessage.warning("请选择年月");
    return;
  }

  if (selectedRoomIds.value.length === 0) {
    ElMessage.warning("请选择至少一个房间");
    return;
  }

  generating.value = true;
  try {
    // Generate bills
    const req: GenerateBillsRequest = {
      year_month: yearMonth.value,
      room_ids: selectedRoomIds.value,
    };

    const result = await billService.generate(req);
    if (result.success) {
      const skipped = selectedRoomIds.value.length - result.generated_count;
      if (skipped > 0) {
        ElMessage.warning(`成功生成 ${result.generated_count} 张账单，${skipped} 个房间因缺少水电读数被跳过（请先在抄表录入中登记读数）`);
      } else {
        ElMessage.success(`成功生成 ${result.generated_count} 张账单`);
      }
      await roomStore.fetchRooms();
      initMeterReadings();
      selectedRoomIds.value = [];
    } else {
      ElMessage.error(result.message || "生成失败");
    }
  } catch (e) {
    ElMessage.error("生成账单失败");
    console.error(e);
  } finally {
    generating.value = false;
  }
}
</script>

<template>
  <div class="bill-generate">
    <!-- 工具栏 -->
    <el-row :gutter="16" class="toolbar">
      <el-col :span="8">
        <el-date-picker
          v-model="yearMonth"
          type="month"
          placeholder="选择年月"
          format="YYYY[年]MM[月]"
          value-format="YYYY-MM"
          style="width: 100%"
        />
      </el-col>
      <el-col :span="16" class="toolbar-buttons">
        <el-button @click="initMeterReadings">重置读数</el-button>
        <el-button
          type="primary"
          :loading="generating"
          :disabled="selectedRoomIds.length === 0"
          @click="handleGenerate"
        >
          生成账单
        </el-button>
      </el-col>
    </el-row>

    <!-- 说明 -->
    <el-alert type="info" :closable="false" class="tips">
      生成账单前，请先在抄表录入中登记本月水电表读数。账单将基于 meter_readings 表中的读数计算水电费。
    </el-alert>

    <!-- 房间列表 -->
    <el-table
      :data="rentableRooms"
      v-loading="loading"
      stripe
      class="room-meter-table"
      @selection-change="(val: any) => (selectedRoomIds = val.map((r: any) => r.id))"
    >
      <el-table-column type="selection" width="55" />
      <el-table-column prop="room_number" label="房间号" width="120" />
      <el-table-column prop="status" label="状态" width="100">
        <template #default="{ row }">
          <el-tag :type="row.status === '在租' ? 'success' : 'info'">
            {{ row.status }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="上次水表" width="120">
        <template #default="{ row }">
          {{ row.water_meter_current || 0 }}
        </template>
      </el-table-column>
      <el-table-column label="本次水表" width="160">
        <template #default="{ row }">
          <el-input-number
            :model-value="getMeterReading(row.id)?.water_reading"
            @update:model-value="
              (val: number | undefined) => {
                const reading = getMeterReading(row.id);
                if (reading) reading.water_reading = val ?? 0;
              }
            "
            :min="0"
            :step="1"
            size="small"
            style="width: 120px"
          />
        </template>
      </el-table-column>
      <el-table-column label="上次电表" width="120">
        <template #default="{ row }">
          {{ row.electric_meter_current || 0 }}
        </template>
      </el-table-column>
      <el-table-column label="本次电表" width="160">
        <template #default="{ row }">
          <el-input-number
            :model-value="getMeterReading(row.id)?.electric_reading"
            @update:model-value="
              (val: number | undefined) => {
                const reading = getMeterReading(row.id);
                if (reading) reading.electric_reading = val ?? 0;
              }
            "
            :min="0"
            :step="1"
            size="small"
            style="width: 120px"
          />
        </template>
      </el-table-column>
      <el-table-column label="月租金(元)" width="120">
        <template #default="{ row }">
          {{ formatMoney(row.base_rent) }}
        </template>
      </el-table-column>
    </el-table>

    <div v-if="rentableRooms.length === 0 && !loading" class="empty-tip">
      <el-empty description="暂无可生成账单的房间" />
    </div>
  </div>
</template>

<style scoped lang="scss">
.bill-generate {
  .toolbar {
    margin-top: 16px;
    margin-bottom: 16px;
    align-items: center;

    .toolbar-buttons {
      display: flex;
      justify-content: flex-end;
      gap: 8px;
    }
  }

  .tips {
    margin-bottom: 16px;
  }

  .room-meter-table {
    margin-top: 8px;
  }

  .empty-tip {
    margin-top: 40px;
  }
}
</style>
