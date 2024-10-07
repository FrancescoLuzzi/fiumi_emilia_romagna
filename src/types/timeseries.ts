export type TimeValue = {
  t: number;
  v?: number;
};

export type TimeValueWithThresholds = {
  time: number;
  value: number;
  soglia1: number;
  soglia2: number;
  soglia3: number;
};

export type TimeSeries = TimeValue[];
export type TimeSeriesWithThresholds = TimeValueWithThresholds[];
