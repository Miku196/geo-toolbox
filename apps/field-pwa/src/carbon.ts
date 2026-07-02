/// Carbon stock estimation engine (IPCC Tier 1, client-side).

// IPCC Tier 1 default carbon densities (tC/ha) by land class and climate zone
const CARBON_DENSITIES: Record<string, { agb: number; bgb: number; soil: number }> = {
  forest:    { agb: 80, bgb: 20, soil: 60 },
  grassland: { agb: 4,  bgb: 8,  soil: 70 },
  wetland:   { agb: 30, bgb: 10, soil: 140 },
  cropland:  { agb: 3,  bgb: 1,  soil: 50 },
  shrubland: { agb: 20, bgb: 8,  soil: 40 },
  bare:      { agb: 0,  bgb: 0,  soil: 10 },
  water:     { agb: 0,  bgb: 0,  soil: 5 },
  built_up:  { agb: 1,  bgb: 0.5, soil: 20 },
};

export interface CarbonResult {
  areaHa: number;
  landClass: string;
  agbTc: number;
  bgbTc: number;
  soilTc: number;
  totalTc: number;
  totalTco2e: number;
}

/**
 * Estimate total carbon stock for a given land class and area.
 * CO₂ equivalent = total carbon × (44/12).
 */
export function estimateCarbonStock(landClass: string, areaHa: number): CarbonResult {
  const densities = CARBON_DENSITIES[landClass] ?? CARBON_DENSITIES.grassland;
  const agbTc = densities.agb * areaHa;
  const bgbTc = densities.bgb * areaHa;
  const soilTc = densities.soil * areaHa;
  const totalTc = agbTc + bgbTc + soilTc;

  return {
    areaHa: Math.round(areaHa * 100) / 100,
    landClass,
    agbTc: Math.round(agbTc * 10) / 10,
    bgbTc: Math.round(bgbTc * 10) / 10,
    soilTc: Math.round(soilTc * 10) / 10,
    totalTc: Math.round(totalTc * 10) / 10,
    totalTco2e: Math.round(totalTc * 44 / 12 * 10) / 10,
  };
}

/**
 * Estimate annual carbon sequestration rate (tCO₂/ha/yr) by land class.
 */
export function annualSequestrationRate(landClass: string): number {
  const rates: Record<string, number> = {
    forest: 4.8, grassland: 1.2, wetland: 8.5,
    cropland: 0.5, shrubland: 2.0, bare: 0.1, water: 0.0, built_up: 0.0,
  };
  return rates[landClass] ?? 0;
}
