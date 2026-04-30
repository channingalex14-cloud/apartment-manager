path = r'd:\vibe coding\Apartment\A1\apartment-manager\src-tauri\src\services\bill_service.rs'
content = open(path, 'rb').read().decode('utf-8', errors='replace')
# Normalize line endings
content = content.replace('\r\n', '\n').replace('\r', '\n')

# 1. Add BillSummary struct after MIDDLE_DAY const
old1 = 'const MIDDLE_DAY: i64 = 15;\n\n// ========================\n// 账单查询响应类型\n// ========================\n\n/// 账单列表项'
new1 = ('const MIDDLE_DAY: i64 = 15;\n\n'
    '#[derive(Debug, Clone, Serialize, Deserialize)]\n'
    'pub struct BillSummary {\n'
    '    pub total_amount: i64,\n'
    '    pub total_paid: i64,\n'
    '    pub total_pending: i64,\n'
    '    pub bill_count: i64,\n'
    '    pub pending_count: i64,\n'
    '    pub paid_count: i64,\n'
    '}\n\n'
    '// ========================\n'
    '// 账单查询响应类型\n'
    '// ========================\n\n'
    '/// 账单列表项（分页）')
content = content.replace(old1, new1, 1)
print('BillSummary added' if old1 not in content else 'FAILED - BillSummary')

# 2. Fix partial_pay_bill_tx call
old2 = 'queries::partial_pay_bill_tx(tx, bill_id, amount)'
new2 = 'queries::partial_pay_bill_tx(tx, bill_id, amount, new_status.as_str())'
content = content.replace(old2, new2, 1)
print('partial_pay_bill_tx fixed' if old2 not in content else 'FAILED - partial_pay_bill_tx')

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)
print('File written')
