#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
成都开发区碳收支 — QGIS 项目生成器
生成可直接在 QGIS Desktop 中打开的 .qgs 项目文件，
包含四个开发区的边界、标注、和适合打印的布局。

也生成对应的 qgis_process 批处理脚本。
"""

import json
from pathlib import Path

DATA_DIR = Path(__file__).parent
QGIS_BIN = "qgis_process"  # 或 "C:/Program Files/QGIS 3.34/bin/qgis_process-qgis.bat"

# ── 生成 QGIS 项目文件 (.qgs) ──────────────────────

def gen_qgis_project():
    """生成最小可用的 QGIS 项目 XML"""
    zones_path = (DATA_DIR / "chengdu-zones.geojson").as_posix()
    
    qgs = f"""<!DOCTYPE qgis PUBLIC 'http://mrcc.com/qgis.dtd' 'SYSTEM'>
<qgis projectname="成都开发区碳收支" version="3.34.0">
  <mapcanvas name="theMapCanvas">
    <destinationsrs>
      <spatialrefsys>
        <wkt>PROJCRS["WGS 84 / Pseudo-Mercator",
    BASEGEOGCRS["WGS 84",ENSEMBLE["World Geodetic System 1984 ensemble",
        MEMBER["World Geodetic System 1984 (Transit)"],
        MEMBER["World Geodetic System 1984 (G730)"],
        MEMBER["World Geodetic System 1984 (G873)"],
        MEMBER["World Geodetic System 1984 (G1150)"],
        MEMBER["World Geodetic System 1984 (G1674)"],
        MEMBER["World Geodetic System 1984 (G1762)"],
        MEMBER["World Geodetic System 1984 (G2139)"],
        ELLIPSOID["WGS 84",6378137,298.257223563,LENGTHUNIT["metre",1]],ENSEMBLEACCURACY[2.0]],PRIMEM["Greenwich",0,ANGLEUNIT["degree",0.0174532925199433]],ID["EPSG",4326]],
    CONVERSION["Popular Visualisation Pseudo-Mercator",METHOD["Popular Visualisation Pseudo Mercator",ID["EPSG",1024]],PARAMETER["Latitude of natural origin",0,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8801]],PARAMETER["Longitude of natural origin",0,ANGLEUNIT["degree",0.0174532925199433],ID["EPSG",8802]],PARAMETER["False easting",0,LENGTHUNIT["metre",1],ID["EPSG",8806]],PARAMETER["False northing",0,LENGTHUNIT["metre",1],ID["EPSG",8807]]],
    CS[Cartesian,2],AXIS["easting (X)",east,ORDER[1],LENGTHUNIT["metre",1]],AXIS["northing (Y)",north,ORDER[2],LENGTHUNIT["metre",1]],ID["EPSG",3857]]</wkt>
        <proj4>+proj=merc +a=6378137 +b=6378137 +lat_ts=0 +lon_0=0 +x_0=0 +y_0=0 +k=1 +units=m +nadgrids=@null +wktext +no_defs</proj4>
        <srsid>3857</srsid>
        <srid>3857</srid>
        <authid>EPSG:3857</authid>
        <description>WGS 84 / Pseudo-Mercator</description>
        <projectionacronym>merc</projectionacronym>
        <ellipsoidacronym>EPSG:7030</ellipsoidacronym>
        <geographicflag>false</geographicflag>
      </spatialrefsys>
    </destinationsrs>
    <extent>
      <xmin>104.0</xmin><ymin>30.25</ymin><xmax>104.55</xmax><ymax>30.65</ymax>
    </extent>
  </mapcanvas>
  <projectlayers>
    <maplayer type="vector" autoRefreshTime="0" refreshOnNotifyEnabled="0" legend-placeholder-image="0">
      <id>chengdu_zones_2025</id>
      <datasource>{zones_path}</datasource>
      <layername>成都开发区</layername>
      <provider>ogr</provider>
      <renderer-v2 type="categorizedSymbol" attr="name" forceraster="0" symbollevels="0" referencescale="-1">
        <categories>
          <category render="true" symbol="0" value="成都高新区" type="string" label="成都高新区"/>
          <category render="true" symbol="1" value="天府新区 (成都片区)" type="string" label="天府新区"/>
          <category render="true" symbol="2" value="成都经开区" type="string" label="成都经开区"/>
          <category render="true" symbol="3" value="成都东部新区" type="string" label="成都东部新区"/>
        </categories>
        <symbols>
          <symbol type="fill" name="0" alpha="0.4" clip_to_extent="1" force_rhr="0">
            <layer class="SimpleFill" locked="0" enabled="1">
              <prop k="color" v="230,50,50,255"/>
              <prop k="outline_color" v="180,0,0,255"/>
              <prop k="outline_width" v="0.8"/>
            </layer>
          </symbol>
          <symbol type="fill" name="1" alpha="0.4" clip_to_extent="1" force_rhr="0">
            <layer class="SimpleFill" locked="0" enabled="1">
              <prop k="color" v="50,150,230,255"/>
              <prop k="outline_color" v="0,100,180,255"/>
              <prop k="outline_width" v="0.8"/>
            </layer>
          </symbol>
          <symbol type="fill" name="2" alpha="0.4" clip_to_extent="1" force_rhr="0">
            <layer class="SimpleFill" locked="0" enabled="1">
              <prop k="color" v="230,180,50,255"/>
              <prop k="outline_color" v="180,130,0,255"/>
              <prop k="outline_width" v="0.8"/>
            </layer>
          </symbol>
          <symbol type="fill" name="3" alpha="0.4" clip_to_extent="1" force_rhr="0">
            <layer class="SimpleFill" locked="0" enabled="1">
              <prop k="color" v="50,200,100,255"/>
              <prop k="outline_color" v="0,150,50,255"/>
              <prop k="outline_width" v="0.8"/>
            </layer>
          </symbol>
        </symbols>
      </renderer-v2>
      <labeling type="simple">
        <settings>
          <text-style fontFamily="Microsoft YaHei" fontSize="10" fontWeight="63" namedStyle="Bold">
            <text-color r="40" g="40" b="40"/>
          </text-style>
          <placement placement="1" centroidWhole="1" offsetUnits="MM" offset="1"/>
        </settings>
      </labeling>
    </maplayer>
  </projectlayers>
  <layer-tree-group>
    <customproperties/>
    <layer-tree-layer id="chengdu_zones_2025" source="{zones_path}" name="成都开发区" checked="Qt::Checked" expanded="1" providerKey="ogr">
      <customproperties/>
    </layer-tree-layer>
  </layer-tree-group>
</qgis>"""
    
    out_path = DATA_DIR / "chengdu-carbon-zones.qgs"
    out_path.write_text(qgs, encoding="utf-8")
    print(f"QGIS 项目文件已生成: {out_path}")
    return out_path


# ── 生成 qgis_process 批处理脚本 ──────────────────

def gen_qgis_process_scripts():
    """生成可用于 qgis_process 的批处理命令"""
    
    scripts = []
    
    # 1. 重投影到等积投影 (面积准确, 碳核算必需)
    scripts.append((
        "重投影到等积投影 EPSG:3405",
        f"""{QGIS_BIN} run native:reprojectlayer \\
  --INPUT={DATA_DIR.as_posix()}/chengdu-zones.geojson \\
  --TARGET_CRS=EPSG:3405 \\
  --OUTPUT={DATA_DIR.as_posix()}/chengdu-zones-equalarea.gpkg"""
    ))
    
    # 2. 计算各开发区精确面积
    scripts.append((
        "计算精确面积并添加字段",
        f"""{QGIS_BIN} run native:fieldcalculator \\
  --INPUT={DATA_DIR.as_posix()}/chengdu-zones-equalarea.gpkg \\
  --FIELD_NAME=area_m2 \\
  --FIELD_TYPE=0 \\
  --FIELD_LENGTH=18 \\
  --FIELD_PRECISION=2 \\
  --FORMULA='$area' \\
  --OUTPUT={DATA_DIR.as_posix()}/chengdu-zones-with-area.gpkg"""
    ))
    
    # 3. 缓冲区分析 (开发区周边影响带)
    scripts.append((
        "开发区 2km 影响缓冲区",
        f"""{QGIS_BIN} run native:buffer \\
  --INPUT={DATA_DIR.as_posix()}/chengdu-zones-equalarea.gpkg \\
  --DISTANCE=2000 \\
  --DISSOLVE=1 \\
  --OUTPUT={DATA_DIR.as_posix()}/chengdu-zones-buffer2km.gpkg"""
    ))
    
    # 4. 如果有土地覆被数据, 做相交分析
    scripts.append((
        "开发区 × 土地覆被相交 (需土地覆被图层)",
        f"""{QGIS_BIN} run native:intersection \\
  --INPUT={DATA_DIR.as_posix()}/chengdu-zones-equalarea.gpkg \\
  --OVERLAY=<landcover-layer-path> \\
  --OUTPUT={DATA_DIR.as_posix()}/chengdu-zones-landcover.gpkg"""
    ))
    
    # 5. zonal statistics (如果有栅格数据)
    scripts.append((
        "分区碳密度统计 (需碳密度栅格)",
        f"""{QGIS_BIN} run native:zonalstatisticsfb \\
  --INPUT={DATA_DIR.as_posix()}/chengdu-zones-equalarea.gpkg \\
  --INPUT_RASTER=<carbon-density-raster.tif> \\
  --RASTER_BAND=1 \\
  --COLUMN_PREFIX=carbon_ \\
  --STATISTICS='mean,sum,count' \\
  --OUTPUT={DATA_DIR.as_posix()}/chengdu-zones-carbonstats.gpkg"""
    ))
    
    print("\nQGIS Processing 脚本 (复制粘贴到终端执行):")
    print("=" * 60)
    for title, cmd in scripts:
        print(f"\n# {title}")
        print(cmd)
    
    # 对应 geo-toolbox 命令
    print(f"\n{'=' * 60}")
    print("等价 geo-toolbox 命令 (需开启 qgis feature):")
    print(f"{'=' * 60}")
    print("""
# 重投影
geo-toolbox process qgis batch \\
  --algorithm native:reprojectlayer \\
  --input examples/chengdu-carbon/chengdu-zones.geojson \\
  --output chengdu-zones-equalarea.gpkg \\
  --extra '[["TARGET_CRS","EPSG:3405"]]'

# 缓冲区
geo-toolbox process qgis batch \\
  --algorithm native:buffer \\
  --input chengdu-zones-equalarea.gpkg \\
  --output chengdu-zones-buffer2km.gpkg \\
  --extra '[["DISTANCE","2000"],["DISSOLVE","1"]]'

# 相交分析 (需土地覆被图层)
geo-toolbox process qgis batch \\
  --algorithm native:intersection \\
  --input examples/chengdu-carbon/chengdu-zones.geojson \\
  --output chengdu-zones-landcover.gpkg \\
  --extra '[["OVERLAY","<landcover.gpkg>"]]'
""")


# ── 生成 ArcGIS/MapLibre 样式 ─────────────────────

def gen_style_json():
    """生成 MapLibre/DeckGL 样式 (用于 Web 地图)"""
    style = {
        "version": 8,
        "name": "成都开发区碳收支",
        "sources": {
            "chengdu-zones": {
                "type": "geojson",
                "data": "chengdu-zones.geojson"
            }
        },
        "layers": [
            {
                "id": "chengdu-zones-fill",
                "type": "fill",
                "source": "chengdu-zones",
                "paint": {
                    "fill-color": ["match", ["get", "id"],
                        "cd-gaoxin", "#E63232",
                        "cd-tianfu", "#3296E6",
                        "cd-jingkai", "#E6B432",
                        "cd-dongbu", "#32C864",
                        "#999999"],
                    "fill-opacity": 0.4
                }
            },
            {
                "id": "chengdu-zones-outline",
                "type": "line",
                "source": "chengdu-zones",
                "paint": {"line-color": "#333", "line-width": 2}
            },
            {
                "id": "chengdu-zones-labels",
                "type": "symbol",
                "source": "chengdu-zones",
                "layout": {
                    "text-field": ["get", "name"],
                    "text-size": 12
                },
                "paint": {"text-color": "#222"}
            }
        ]
    }
    out_path = DATA_DIR / "chengdu-carbon-style.json"
    out_path.write_text(json.dumps(style, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"\nMapLibre 样式文件已生成: {out_path}")


if __name__ == "__main__":
    gen_qgis_project()
    gen_style_json()
    gen_qgis_process_scripts()
