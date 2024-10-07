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

export const getSoglia1 = (station: Station): number => {
  return station.soglia1 || station.soglia2 || station.soglia3;
};

export const getSoglia2 = (station: Station): number => {
  return station.soglia2 || station.soglia3 || station.soglia1;
};

export const getSoglia3 = (station: Station): number => {
  return station.soglia3 || station.soglia2 || station.soglia1;
};
