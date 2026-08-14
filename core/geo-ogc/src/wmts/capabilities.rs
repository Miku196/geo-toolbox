use super::tile_matrix::{TileMatrixSet, WmtsLayer};
use super::WmtsService;

impl WmtsService {
    /// Build WMTS 1.0.0 GetCapabilities XML document.
    pub fn build_capabilities_xml(&self) -> String {
        let layers_xml: String = self
            .layers
            .iter()
            .map(|l| self.layer_to_xml(l))
            .collect::<Vec<_>>()
            .join("
");

        let tms_xml: String = self
            .tile_matrix_sets
            .iter()
            .map(|t| self.tile_matrix_set_to_xml(t))
            .collect::<Vec<_>>()
            .join("
");

        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Capabilities version="1.0.0"
              xmlns="http://www.opengis.net/wmts/1.0"
              xmlns:ows="http://www.opengis.net/ows/1.1"
              xmlns:xlink="http://www.w3.org/1999/xlink"
              xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <ows:ServiceIdentification>
    <ows:Title>{title}</ows:Title>
    <ows:ServiceType>OGC WMTS</ows:ServiceType>
    <ows:ServiceTypeVersion>1.0.0</ows:ServiceTypeVersion>
  </ows:ServiceIdentification>
  <ows:ServiceProvider>
    <ows:ProviderName>geo-toolbox</ows:ProviderName>
  </ows:ServiceProvider>
  <ows:OperationsMetadata>
    <ows:Operation name="GetCapabilities">
      <ows:DCP>
        <ows:HTTP>
          <ows:Get xlink:href="{url}">
            <ows:Constraint name="GetEncoding">
              <ows:AllowedValues>
                <ows:Value>KVP</ows:Value>
              </ows:AllowedValues>
            </ows:Constraint>
          </ows:Get>
        </ows:HTTP>
      </ows:DCP>
    </ows:Operation>
    <ows:Operation name="GetTile">
      <ows:DCP>
        <ows:HTTP>
          <ows:Get xlink:href="{url}">
            <ows:Constraint name="GetEncoding">
              <ows:AllowedValues>
                <ows:Value>KVP</ows:Value>
              </ows:AllowedValues>
            </ows:Constraint>
          </ows:Get>
        </ows:HTTP>
      </ows:DCP>
    </ows:Operation>
  </ows:OperationsMetadata>
  <Contents>
 {layers_xml}
 {tms_xml}
  </Contents>
  <ServiceMetadataURL xlink:href="{url}"/>
</Capabilities>"#,
            title = self.title,
            url = self.online_resource,
            layers_xml = layers_xml,
            tms_xml = tms_xml,
        )
    }

    fn layer_to_xml(&self, layer: &WmtsLayer) -> String {
        let bbox_xml = if let Some(bbox) = &layer.wgs84_bbox {
            format!(
                r#"      <ows:WGS84BoundingBox>
        <ows:LowerCorner>{west} {south}</ows:LowerCorner>
        <ows:UpperCorner>{east} {north}</ows:UpperCorner>
      </ows:WGS84BoundingBox>"#,
                west = bbox.west,
                south = bbox.south,
                east = bbox.east,
                north = bbox.north,
            )
        } else {
            String::new()
        };

        let formats_xml: String = layer
            .formats
            .iter()
            .map(|f| format!("      <Format>{f}</Format>"))
            .collect::<Vec<_>>()
            .join("
");

        let styles_xml: String = if layer.styles.is_empty() {
            r#"      <Style isDefault="true">
        <ows:Title>Default</ows:Title>
        <ows:Identifier>default</ows:Identifier>
      </Style>"#
                .into()
        } else {
            layer
                .styles
                .iter()
                .map(|s| {
                    format!(
                        r#"      <Style isDefault="false">
        <ows:Title>{s}</ows:Title>
        <ows:Identifier>{s}</ows:Identifier>
      </Style>"#
                    )
                })
                .collect::<Vec<_>>()
                .join("
")
        };

        let tms_refs: String = layer
            .tile_matrix_sets
            .iter()
            .map(|t| format!("      <TileMatrixSetLink>
        <TileMatrixSet>{t}</TileMatrixSet>
      </TileMatrixSetLink>"))
            .collect::<Vec<_>>()
            .join("
");

        let resource_url = if let Some(url) = &layer.resource_url {
            format!(
                r#"    <ResourceURL format="{fmt}" resourceType="tile" template="{url}"/>"#,
                fmt = layer
                    .formats
                    .first()
                    .map(|s| s.as_str())
                    .unwrap_or("image/png"),
                url = url,
            )
        } else {
            String::new()
        };

        format!(
            r#"    <Layer>
      <ows:Title>{title}</ows:Title>
      <ows:Identifier>{name}</ows:Identifier>
 {abstract_xml}
 {bbox_xml}
      <ows:CRS>{crs}</ows:CRS>
 {tms_refs}
 {formats_xml}
 {styles_xml}
 {resource_url}
    </Layer>"#,
            title = layer.title,
            name = layer.name,
            abstract_xml = layer
                .abstract_
                .as_ref()
                .map(|a| format!("      <ows:Abstract>{a}</ows:Abstract>"))
                .unwrap_or_default(),
            bbox_xml = bbox_xml,
            crs = layer.crs.first().map(|s| s.as_str()).unwrap_or("EPSG:4326"),
            tms_refs = tms_refs,
            formats_xml = formats_xml,
            styles_xml = styles_xml,
            resource_url = resource_url,
        )
    }

    fn tile_matrix_set_to_xml(&self, tms: &TileMatrixSet) -> String {
        let matrices_xml: String = tms
            .tile_matrices
            .iter()
            .map(|tm| {
                format!(
                    r#"      <TileMatrix>
        <ows:Identifier>{id}</ows:Identifier>
        <ScaleDenominator>{scale}</ScaleDenominator>
        <TopLeftCorner>{tlx} {tly}</TopLeftCorner>
        <TileWidth>{tw}</TileWidth>
        <TileHeight>{th}</TileHeight>
        <MatrixWidth>{mw}</MatrixWidth>
        <MatrixHeight>{mh}</MatrixHeight>
      </TileMatrix>"#,
                    id = tm.identifier,
                    scale = tm.scale_denominator,
                    tlx = tm.top_left_x,
                    tly = tm.top_left_y,
                    tw = tm.tile_width,
                    th = tm.tile_height,
                    mw = tm.matrix_width,
                    mh = tm.matrix_height,
                )
            })
            .collect::<Vec<_>>()
            .join("
");

        format!(
            r#"    <TileMatrixSet>
      <ows:Identifier>{id}</ows:Identifier>
      <ows:CRS>{crs}</ows:CRS>
      <ows:BoundingBox CRS="{crs}">
        <ows:LowerCorner>{west} {south}</ows:LowerCorner>
        <ows:UpperCorner>{east} {north}</ows:UpperCorner>
      </ows:BoundingBox>
 {matrices_xml}
    </TileMatrixSet>"#,
            id = tms.identifier,
            crs = tms.supported_crs,
            west = tms.bounding_box.west,
            south = tms.bounding_box.south,
            east = tms.bounding_box.east,
            north = tms.bounding_box.north,
            matrices_xml = matrices_xml,
        )
    }
}
