<script setup lang="ts">
import { ref, onMounted } from "vue";
import { ElMessage } from "element-plus";
import { configService } from "@/services/config.service";
import { availableQRImages, getQRImageUrl } from "@/utils/billAssets";

const loading = ref(false);
const saving = ref(false);

const bankForm = ref({
  公寓名称: "",
  楼栋地址: "",
  银行户名: "",
  银行帐号: "",
  开户行: "",
  温馨提示: "",
  收款二维码: "",
});

const bankConfigKeys = [
  '公寓名称', '楼栋地址', '银行户名', '银行帐号', '开户行', '温馨提示', '收款二维码',
];

async function fetchBankInfo() {
  loading.value = true;
  try {
    const configs = await configService.getAll();
    bankConfigKeys.forEach(key => {
      const config = configs.find(c => c.config_key === key);
      bankForm.value[key as keyof typeof bankForm.value] = config?.config_value || '';
    });
  } catch {
    ElMessage.error('获取收款信息失败');
  } finally {
    loading.value = false;
  }
}

async function saveBankInfo() {
  saving.value = true;
  try {
    for (const key of bankConfigKeys) {
      const val = bankForm.value[key as keyof typeof bankForm.value];
      await configService.set(key, val);
    }
    ElMessage.success('收款信息保存成功');
    await fetchBankInfo();
  } catch {
    ElMessage.error('保存失败');
  } finally {
    saving.value = false;
  }
}

onMounted(fetchBankInfo);
</script>

<template>
  <el-card style="margin-top: 20px" v-loading="loading">
    <template #header>
      <div class="card-header">
        <span>收款账户信息</span>
        <el-tag size="small" type="info">收费通知单显示</el-tag>
      </div>
    </template>
    <el-form label-width="100px" class="price-form">
      <el-row :gutter="16">
        <el-col :span="8">
          <el-form-item label="公寓名称">
            <el-input v-model="bankForm.公寓名称" placeholder="如：新逸公寓" />
          </el-form-item>
        </el-col>
        <el-col :span="8">
          <el-form-item label="楼栋地址">
            <el-input v-model="bankForm.楼栋地址" placeholder="如：58栋" />
          </el-form-item>
        </el-col>
        <el-col :span="8">
          <el-form-item label="银行户名">
            <el-input v-model="bankForm.银行户名" placeholder="开户姓名" />
          </el-form-item>
        </el-col>
        <el-col :span="8">
          <el-form-item label="银行帐号">
            <el-input v-model="bankForm.银行帐号" placeholder="卡号" />
          </el-form-item>
        </el-col>
        <el-col :span="8">
          <el-form-item label="开户行">
            <el-input v-model="bankForm.开户行" placeholder="开户支行" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="温馨提示">
        <el-input v-model="bankForm.温馨提示" placeholder="如：请及时缴纳房租水电。逾期收取滞纳金。" />
      </el-form-item>
      <el-form-item label="收款二维码">
        <div class="qr-select">
          <el-select v-model="bankForm.收款二维码" placeholder="选择收款二维码图片" clearable style="width: 300px">
            <el-option v-for="img in availableQRImages" :key="img" :label="img" :value="img" />
          </el-select>
          <span class="qr-hint">
            请将二维码图片放入项目 <code>public/bill-assets/</code> 目录
          </span>
        </div>
        <div v-if="bankForm.收款二维码" class="qr-preview">
          <img :src="getQRImageUrl(bankForm.收款二维码)" alt="收款二维码预览" />
        </div>
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="saveBankInfo" :loading="saving">保存收款信息</el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<style scoped lang="scss">
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.price-form {
  .unit {
    margin-left: 8px;
    font-size: 13px;
    color: var(--el-text-color-secondary);
  }
}

.qr-select {
  display: flex;
  align-items: center;
  gap: 12px;

  .qr-hint {
    font-size: 12px;
    color: var(--el-text-color-secondary);

    code {
      background: #f0f0f0;
      padding: 1px 5px;
      border-radius: 3px;
      font-size: 11px;
      color: #666;
    }
  }
}

.qr-preview {
  margin-top: 12px;
  img {
    width: 160px;
    height: 160px;
    object-fit: contain;
    border: 1px solid #e0e0e0;
    border-radius: 8px;
    background: #fff;
  }
}
</style>
