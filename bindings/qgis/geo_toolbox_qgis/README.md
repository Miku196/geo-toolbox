# Geo Toolbox QGIS Plugin

QGIS 插件，基于 Rust/PyO3 的 geo-toolbox 后端提供浏览器端地理空间处理能力。

## 功能

- **CRS 坐标转换** — WGS84 ↔ GCJ-02 ↔ BD-09 ↔ Web Mercator
- **碳核算** — IPCC 排放计算，支持 GeoJSON + 因子表
- **瓦片数学** — 经纬度 ↔ tile(x,y,z)、瓦片 URL、MVT 编码
- **空间运算** — 面积、bbox、质心、简化 (RDP)、凸包、缓冲区、相交
- **IO** — CSV 解析、GeoJSON/Excel 导出、碳报告 Markdown
- **NMEA 解析** — GGA/RMC/GLL/VTG 语句解析与验证
- **Geohash** — 编码、解码、邻居查询
- **时序** — 日期差、季节判断
- **统计** — 基本统计量、分区统计

## 安装

### 1. 安装 geo-toolbox Python 包

```bash
pip install geo-toolbox
```

### 2. 安装 QGIS 插件

**方法 A — QGIS Plugin Manager（推荐）**

1. 将 `geo_toolbox_qgis.zip` 放到 QGIS 插件目录：
   - Windows: `%APPDATA%\QGIS\QGIS3\profiles\default\python\plugins\`
   - macOS: `~/Library/Application Support/QGIS/QGIS3/profiles/default/python/plugins/`
   - Linux: `~/.local/share/QGIS/QGIS3/profiles/default/python/plugins/`
2. 重启 QGIS
3. 菜单：Plugins → Manage and Install Plugins → 搜索 "Geo Toolbox" → 启用

**方法 B — 手动安装（开发）**

```bash
cd bindings/qgis
make install   # 创建符号链接到 QGIS 插件目录
```

### 3. 构建插件 zip

```bash
cd bindings/qgis
make zip
# → geo_toolbox_qgis.zip
```

## 使用

1. QGIS 工具栏出现 **Geo Toolbox** 按钮（绿色地球图标）
2. 点击按钮打开/关闭侧边面板
3. 从下拉菜单选择工具
4. 输入 JSON 参数并点击 **▶ Execute**
5. 结果在下方面板显示

## 开发

```bash
# 生成图标 SVG
python resources.py

# 运行测试
python -m pytest tests/
```

## 依赖

- QGIS ≥ 3.22
- geo-toolbox ≥ 0.1.0
