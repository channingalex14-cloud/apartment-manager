/**
 * 房间状态管理
 */

import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { roomService } from "@/services/room.service";
import type { RoomResponse } from "@/types/room";
import { createBatchUpdater } from "@/utils/batch";

export const useRoomStore = defineStore("room", () => {
  // 状态
  const rooms = ref<RoomResponse[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  // 计算属性
  const totalCount = computed(() => rooms.value.length);

  const statusCounts = computed(() => {
    const counts: Record<string, number> = { '在租': 0, '空房': 0, '员工': 0, '管理': 0, '违约': 0, '新租': 0, '待清洁': 0, '维修中': 0 };
    for (const r of rooms.value) {
      counts[r.status] = (counts[r.status] || 0) + 1;
    }
    return counts;
  });

  const rentedCount = computed(() => statusCounts.value['在租'] || 0);
  const vacantCount = computed(() => statusCounts.value['空房'] || 0);
  const staffCount = computed(() => statusCounts.value['员工'] || 0);
  const managementCount = computed(() => statusCounts.value['管理'] || 0);
  const violationCount = computed(() => statusCounts.value['违约'] || 0);

  // 按状态分组的房间
  const roomsByStatus = computed(() => {
    const groups: Record<string, RoomResponse[]> = {};
    for (const room of rooms.value) {
      const status = room.status;
      if (!groups[status]) {
        groups[status] = [];
      }
      groups[status]!.push(room);
    }
    return groups;
  });

  // 按楼层分组的房间
  const roomsByFloor = computed(() => {
    const groups: Record<number, RoomResponse[]> = {};
    for (const room of rooms.value) {
      const floor = room.floor || 0;
      if (!groups[floor]) {
        groups[floor] = [];
      }
      groups[floor]!.push(room);
    }
    return groups;
  });

  // 批量更新器：合并 16ms 窗口内的多次刷新请求
  // 防止高频刷新导致 UI 抖动（如快速连续调用 fetchRooms 时只渲染一次）
  const batchRoomsUpdate = createBatchUpdater<RoomResponse[]>(
    (newRoomsList) => {
      // 去重合并
      const seen = new Set<number>();
      rooms.value = newRoomsList.filter((r) => {
        if (seen.has(r.id)) return false;
        seen.add(r.id);
        return true;
      });
    },
    16, // 约一帧的时间
  );

  // 操作
  async function fetchRooms() {
    loading.value = true;
    error.value = null;
    try {
      const list = await roomService.list();
      // 通过批量更新器合并到下一个 16ms 窗口
      batchRoomsUpdate.set(list);
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      console.error("获取房间列表失败:", e);
    } finally {
      loading.value = false;
    }
  }

  async function updateRoom(
    id: number,
    data: {
      base_rent?: number;
      property_fee?: number;
      water_meter_current?: number;
      electric_meter_current?: number;
    }
  ) {
    // 获取当前版本号用于乐观锁检查
    const room = rooms.value.find((r) => r.id === id);
    if (!room) {
      throw new Error(`房间 ${id} 不存在`);
    }

    const success = await roomService.update(id, { version: room.version, ...data });
    if (success) {
      // 刷新列表
      await fetchRooms();
    }
    return success;
  }

  async function updateRoomStatus(id: number, status: string, operator?: string, notes?: string) {
    const success = await roomService.updateStatus(id, status, operator, notes);
    if (success) {
      await fetchRooms();
    }
    return success;
  }

  function getRoomById(id: number): RoomResponse | undefined {
    return rooms.value.find((r) => r.id === id);
  }

  return {
    // 状态
    rooms,
    loading,
    error,
    // 计算属性
    totalCount,
    rentedCount,
    vacantCount,
    staffCount,
    managementCount,
    violationCount,
    roomsByStatus,
    roomsByFloor,
    // 操作
    fetchRooms,
    updateRoom,
    updateRoomStatus,
    getRoomById,
  };
});
