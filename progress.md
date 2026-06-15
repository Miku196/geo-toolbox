# Progress

## 2026-06-16: RUSLE + SCS-CN + InVEST 模块

### 已完成
- [x] P1: RUSLE 土壤流失方程 (16 tests, 2 CLI tools)
  - A = R × K × LS × C × P，5 因子完整计算
  - 侵蚀等级分类（5 级），NDVI→C 因子，DEM→LS 因子
  - 等高/带状/梯田 P 因子查表
- [x] P1: SCS-CN 径流曲线数 (9 tests, 2 CLI tools)
  - 26 种土地利用 × 4 土壤分组 CN 查表表
  - Hawkins AMC 干旱/正常/湿润修正
  - 栅格化径流评估
- [x] P1: InVEST 碳存储 + 水源涵养 (10 tests, 3 CLI tools)
  - 4 碳库（地上/地下/土壤/枯落物），20 种生态系统碳密度
  - Budyko 蒸散发曲线，产水量计算
  - 综合 InvestAssessment

### 统计
- 3 新模块: rusle.rs → ecology plugin, scs_cn.rs + invest.rs → hydro plugin
- 35 新测试全部通过
- 7 新 CLI 工具注册
- ecology 20 测试, hydro 27 测试 (总计 47)

## 2026-06-15: P0 survey 坐标换带 + tools注册

### 已完成
- [x] P0: survey 坐标换带 (Gauss-Krüger 正算/反算/换带/带号检测)
- [x] P0: survey 4新工具注册 (gauss_forward/inverse/zone_transform/zone_info)
- [x] P0: carbon 5-pool/scenario/VCS工具注册
- [x] P0: geohazard FS安全系数+Newmark工具注册
- [x] P0: ecology NDVI变化检测工具注册

### 待做
- [ ] P2: 信息量模型 + ID曲线
- [ ] P2: 随机森林 LULC
- [ ] P2: 高斯烟羽 + CCER报告
- [ ] P3: QGIS工具箱 / Jupyter
