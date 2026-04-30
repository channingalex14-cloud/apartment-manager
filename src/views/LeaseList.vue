<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { ElMessage, ElMessageBox } from "element-plus";
import { useRoomStore } from "@/stores/room";
import { useTenantStore } from "@/stores/tenant";
import { leaseService } from "@/services/lease.service";
import type { Lease } from "@/types/lease";
import { formatMoney } from "@/utils/money";
import { today } from "@/utils/date";

const roomStore = useRoomStore();
const tenantStore = useTenantStore();

const leases = ref<Lease[]>([]);
const loading = ref(false);
const statusFilter = ref("");
const dialogVisible = ref(false);
const checkInDialogVisible = ref(false);
const detailDialogVisible = ref(false);
const detailLease = ref<Lease | null>(null);

const createForm = ref({
  room_id: 0,
  tenant_id: 0,
  start_date: today(),
  end_date: "",
  monthly_rent: 0,
  property_fee: 0,
  deposit: 0,
  contract_number: "",
});

const checkInForm = ref({
  lease_id: 0,
  room_id: 0,
  tenant_id: 0,
  move_in_date: today(),
});

const statusOptions = [
  { label: "全部", value: "" },
  { label: "草稿", value: "草稿" },
  { label: "生效中", value: "生效中" },
  { label: "违约中", value: "违约中" },
  { label: "待结算", value: "待结算" },
  { label: "已退房", value: "已退房" },
];

onMounted(async () => {
  await Promise.all([fetchLeases(), roomStore.fetchRooms(), tenantStore.fetchTenants()]);
});

async function fetchLeases() {
  loading.value = true;
  try {
    leases.value = await leaseService.list();
  } catch {
    ElMessage.error("获取合同列表失败");
  } finally {
    loading.value = false;
  }
}

const filteredLeases = computed(() => {
  if (!statusFilter.value) return leases.value;
  return leases.value.filter((l) => l.status === statusFilter.value);
});

const leaseStats = computed(() => {
  const counts = { total: leases.value.length, active: 0, violation: 0, pending: 0 };
  for (const l of leases.value) {
    if (l.status === "生效中") counts.active++;
    else if (l.status === "违约中") counts.violation++;
    else if (l.status === "待结算") counts.pending++;
  }
  return counts;
});

function getStatusType(status: string): "primary" | "success" | "warning" | "info" | "danger" {
  const map: Record<string, "primary" | "success" | "warning" | "info" | "danger"> = {
    草稿: "info",
    生效中: "success",
    违约中: "danger",
    待结算: "warning",
    已退房: "info",
    已作废: "danger",
  };
  return map[status] || "info";
}

function getRoomNumber(roomId: number): string {
  const room = roomStore.getRoomById(roomId);
  return room?.room_number || String(roomId);
}

function getTenantName(tenantId: number): string {
  const tenant = tenantStore.getTenantById(tenantId);
  return tenant?.name || String(tenantId);
}

function openCreateDialog() {
  createForm.value = {
    room_id: 0,
    tenant_id: 0,
    start_date: today(),
    end_date: "",
    monthly_rent: 200000,
    property_fee: 5000,
    deposit: 200000,
    contract_number: "",
  };
  dialogVisible.value = true;
}

async function handleCreate() {
  try {
    const id = await leaseService.create(createForm.value);
    ElMessage.success("创建成功，合同号：" + id);
    dialogVisible.value = false;
    await fetchLeases();
  } catch {
    ElMessage.error("创建失败");
  }
}

function openCheckInDialog(lease: Lease) {
  checkInForm.value = {
    lease_id: lease.id,
    room_id: lease.room_id,
    tenant_id: lease.tenant_id,
    move_in_date: today(),
  };
  checkInDialogVisible.value = true;
}

async function handleCheckIn() {
  try {
    const result = await leaseService.checkIn(checkInForm.value);
    if (result.success) {
      ElMessage.success("入住成功");
      checkInDialogVisible.value = false;
      await fetchLeases();
      await roomStore.fetchRooms();
    } else {
      ElMessage.error(result.message || "入住失败");
    }
  } catch {
    ElMessage.error("入住失败");
  }
}

function openCheckOutDialog(lease: Lease) {
  ElMessageBox.confirm(
    `确定要为房间 ${getRoomNumber(lease.room_id)} 办理退房吗？`,
    "退房确认",
    { type: "warning" }
  )
    .then(async () => {
      try {
        const result = await leaseService.checkOut({
          lease_id: lease.id,
          room_id: lease.room_id,
          move_out_date: today(),
          reason: "正常退房",
        });
        if (result.success) {
          ElMessage.success("退房成功");
          await fetchLeases();
          await roomStore.fetchRooms();
        } else {
          ElMessage.error(result.message || "退房失败");
        }
      } catch {
        ElMessage.error("退房失败");
      }
    })
    .catch(() => {});
}

async function handleMarkViolation(lease: Lease) {
  try {
    await ElMessageBox.confirm(
      `确定要将房间 ${getRoomNumber(lease.room_id)} 标记为违约吗？`,
      "违约确认",
      { type: "warning" }
    );
    const result = await leaseService.markViolation(lease.id);
    if (result.success) {
      ElMessage.success("已标记为违约");
      await fetchLeases();
      await roomStore.fetchRooms();
    } else {
      ElMessage.error(result.message || "操作失败");
    }
  } catch {
    // cancelled
  }
}

async function handleRecover(lease: Lease) {
  try {
    await ElMessageBox.confirm(
      `确定要恢复房间 ${getRoomNumber(lease.room_id)} 的正常状态吗？`,
      "恢复确认",
      { type: "warning" }
    );
    const result = await leaseService.recoverFromViolation(lease.id);
    if (result.success) {
      ElMessage.success("已恢复");
      await fetchLeases();
      await roomStore.fetchRooms();
    } else {
      ElMessage.error(result.message || "操作失败");
    }
  } catch {
    // cancelled
  }
}

function openDetailDialog(lease: Lease) {
  detailLease.value = lease;
  detailDialogVisible.value = true;
}
</script>

<template>
  <div class="lease-list">
    <el-row :gutter="16" class="toolbar">
      <el-col :span="6">
        <el-select v-model="statusFilter" placeholder="状态筛选" clearable>
          <el-option v-for="opt in statusOptions" :key="opt.value" :label="opt.label" :value="opt.value" />
        </el-select>
      </el-col>
      <el-col :span="18" class="toolbar-buttons">
        <el-button type="primary" @click="openCreateDialog">新建合同</el-button>
      </el-col>
    </el-row>

    <el-row :gutter="16" class="stats-row">
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stat-item">
            <span class="stat-label">合同总数</span>
            <span class="stat-value">{{ leaseStats.total }}</span>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stat-item">
            <span class="stat-label">生效中</span>
            <span class="stat-value success">{{ leaseStats.active }}</span>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stat-item">
            <span class="stat-label">违约中</span>
            <span class="stat-value danger">{{ leaseStats.violation }}</span>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stat-item">
            <span class="stat-label">待结算</span>
            <span class="stat-value warning">{{ leaseStats.pending }}</span>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-table :data="filteredLeases" v-loading="loading" stripe>
      <el-table-column prop="id" label="ID" width="60" />
      <el-table-column label="房间" width="100">
        <template #default="{ row }">{{ getRoomNumber(row.room_id) }}</template>
      </el-table-column>
      <el-table-column label="租客" width="100">
        <template #default="{ row }">{{ getTenantName(row.tenant_id) }}</template>
      </el-table-column>
      <el-table-column prop="status" label="状态" width="100">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">{{ row.status }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="start_date" label="开始日期" width="120" />
      <el-table-column prop="end_date" label="结束日期" width="120">
        <template #default="{ row }">{{ row.end_date || "-" }}</template>
      </el-table-column>
      <el-table-column label="月租金" width="120">
        <template #default="{ row }">{{ formatMoney(row.monthly_rent) }}</template>
      </el-table-column>
      <el-table-column label="押金余额" width="120">
        <template #default="{ row }">{{ formatMoney(row.deposit_balance) }}</template>
      </el-table-column>
      <el-table-column label="操作" fixed="right" width="300">
        <template #default="{ row }">
          <el-button v-if="row.status === '草稿'" type="primary" size="small" text @click="openCheckInDialog(row)">入住</el-button>
          <el-button v-else-if="row.status === '生效中'" type="warning" size="small" text @click="openCheckOutDialog(row)">退房</el-button>
          <el-button v-if="row.status === '生效中'" type="danger" size="small" text @click="handleMarkViolation(row)">违约</el-button>
          <el-button v-if="row.status === '违约中'" type="success" size="small" text @click="handleRecover(row)">恢复</el-button>
          <el-button type="primary" size="small" text @click="openDetailDialog(row)">详情</el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-dialog v-model="dialogVisible" title="新建合同" width="600px">
      <el-form :model="createForm" label-width="100px">
        <el-form-item label="房间">
          <el-select v-model="createForm.room_id" placeholder="选择房间">
            <el-option v-for="room in roomStore.rooms" :key="room.id" :label="room.room_number" :value="room.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="租客">
          <el-select v-model="createForm.tenant_id" placeholder="选择租客">
            <el-option v-for="tenant in tenantStore.tenants" :key="tenant.id" :label="tenant.name + ' - ' + tenant.phone" :value="tenant.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="合同编号">
          <el-input v-model="createForm.contract_number" placeholder="可不填，系统自动生成" />
        </el-form-item>
        <el-form-item label="开始日期">
          <el-date-picker v-model="createForm.start_date" type="date" placeholder="选择日期" format="YYYY-MM-DD" value-format="YYYY-MM-DD" />
        </el-form-item>
        <el-form-item label="结束日期">
          <el-date-picker v-model="createForm.end_date" type="date" placeholder="可不填" format="YYYY-MM-DD" value-format="YYYY-MM-DD" />
        </el-form-item>
        <el-form-item label="月租金(元)">
          <el-input-number v-model="createForm.monthly_rent" :min="0" :step="100" />
        </el-form-item>
        <el-form-item label="物业费(元)">
          <el-input-number v-model="createForm.property_fee" :min="0" :step="50" />
        </el-form-item>
        <el-form-item label="押金(元)">
          <el-input-number v-model="createForm.deposit" :min="0" :step="100" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleCreate">创建</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="checkInDialogVisible" title="办理入住" width="500px">
      <el-form :model="checkInForm" label-width="100px">
        <el-form-item label="房间">
          <el-input :model-value="getRoomNumber(checkInForm.room_id)" disabled />
        </el-form-item>
        <el-form-item label="租客">
          <el-input :model-value="getTenantName(checkInForm.tenant_id)" disabled />
        </el-form-item>
        <el-form-item label="入住日期">
          <el-date-picker v-model="checkInForm.move_in_date" type="date" placeholder="选择日期" format="YYYY-MM-DD" value-format="YYYY-MM-DD" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="checkInDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleCheckIn">确认入住</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="detailDialogVisible" title="合同详情" width="600px">
      <template v-if="detailLease">
        <el-descriptions :column="2" border>
          <el-descriptions-item label="合同ID">{{ detailLease.id }}</el-descriptions-item>
          <el-descriptions-item label="状态">
            <el-tag :type="getStatusType(detailLease.status)">{{ detailLease.status }}</el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="房间">{{ getRoomNumber(detailLease.room_id) }}</el-descriptions-item>
          <el-descriptions-item label="租客">{{ getTenantName(detailLease.tenant_id) }}</el-descriptions-item>
          <el-descriptions-item label="合同编号">{{ detailLease.contract_number || '-' }}</el-descriptions-item>
          <el-descriptions-item label="开始日期">{{ detailLease.start_date }}</el-descriptions-item>
          <el-descriptions-item label="结束日期">{{ detailLease.end_date || '-' }}</el-descriptions-item>
          <el-descriptions-item label="月租金">{{ formatMoney(detailLease.monthly_rent) }}</el-descriptions-item>
          <el-descriptions-item label="物业费">{{ formatMoney(detailLease.property_fee) }}</el-descriptions-item>
          <el-descriptions-item label="押金">{{ formatMoney(detailLease.deposit) }}</el-descriptions-item>
          <el-descriptions-item label="押金余额">{{ formatMoney(detailLease.deposit_balance) }}</el-descriptions-item>
          <el-descriptions-item label="押金状态">{{ detailLease.deposit_status || '-' }}</el-descriptions-item>
        </el-descriptions>
      </template>
      <template #footer>
        <el-button @click="detailDialogVisible = false">关闭</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped lang="scss">
.lease-list {
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

  .stats-row {
    margin-bottom: 16px;

    .stat-item {
      display: flex;
      flex-direction: column;
      align-items: center;
      padding: 8px 0;

      .stat-label {
        font-size: 14px;
        color: var(--el-text-color-secondary);
        margin-bottom: 4px;
      }

      .stat-value {
        font-size: 24px;
        font-weight: bold;
        color: var(--el-color-primary);

        &.success { color: var(--el-color-success); }
        &.danger { color: var(--el-color-danger); }
        &.warning { color: var(--el-color-warning); }
      }
    }
  }
}
</style>
