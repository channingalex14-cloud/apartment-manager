<script setup lang="ts">
import PriceConfigCard from "@/components/settings/PriceConfigCard.vue";
import BankInfoCard from "@/components/settings/BankInfoCard.vue";
import ConfigTableCard from "@/components/settings/ConfigTableCard.vue";
import MaintenanceCard from "@/components/settings/MaintenanceCard.vue";
import ImportCard from "@/components/settings/ImportCard.vue";
import ArchiveCard from "@/components/settings/ArchiveCard.vue";
import BackupCard from "@/components/settings/BackupCard.vue";
import ExportCard from "@/components/settings/ExportCard.vue";
</script>

<template>
  <div class="settings-page">
    <PriceConfigCard />
    <BankInfoCard />
    <ConfigTableCard />
    <MaintenanceCard />
    <ImportCard />
    <ArchiveCard />
    <BackupCard />
    <ExportCard />
  </div>
</template>

<style scoped lang="scss">
.settings-page {
  padding: 16px;
}
</style>
