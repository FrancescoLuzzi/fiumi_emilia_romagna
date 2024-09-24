export type Station = {
  idstazione: string;
  ordinamento: number;
  nomestaz: string;
  lon: string;
  lat: string;
  value?: number;
  soglia1: number;
  soglia2: number;
  soglia3: number;
};

export type Stations = Station[];
