/**
 * Business Metrics and Monitoring Coverage System
 * 
 * Provides standardized metrics collection, label sanitization, high-cardinality protection,
 * and production visibility for application business metrics.
 */

export type MetricType = 'counter' | 'gauge' | 'histogram';

export interface MetricLabels {
  [key: string]: string | number | boolean | undefined | null;
}

export interface MetricValue {
  name: string;
  type: MetricType;
  value: number;
  labels: Record<string, string>;
  timestamp: number;
}

export interface HistogramBucket {
  le: number;
  count: number;
}

export interface HistogramValue {
  name: string;
  sum: number;
  count: number;
  buckets: HistogramBucket[];
  labels: Record<string, string>;
  timestamp: number;
}

export interface MetricDefinition {
  name: string;
  help: string;
  type: MetricType;
  labelNames?: string[];
  buckets?: number[];
}

export interface MetricsRegistryOptions {
  prefix?: string;
  maxCardinalityPerMetric?: number;
  maxLabelValueLength?: number;
}

const DEFAULT_HISTOGRAM_BUCKETS = [0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10];
const METRIC_NAME_REGEX = /^[a-zA-Z_][a-zA-Z0-9_]*$/;
const LABEL_KEY_REGEX = /^[a-zA-Z_][a-zA-Z0-9_]*$/;

/**
 * Validates metric naming conventions according to Prometheus / OpenTelemetry standards.
 * Enforces:
 * - Valid character regex: ^[a-zA-Z_][a-zA-Z0-9_]*$
 * - Standard suffixes: _total for counters, _seconds/_ms for durations, _bytes for sizes, _ratio for rates.
 */
export function validateMetricName(name: string, type: MetricType): boolean {
  if (!name || typeof name !== 'string') {
    return false;
  }
  if (!METRIC_NAME_REGEX.test(name)) {
    return false;
  }
  if (type === 'counter' && !name.endsWith('_total')) {
    return false;
  }
  if (type === 'histogram' && !(name.endsWith('_seconds') || name.endsWith('_ms') || name.endsWith('_bytes') || name.endsWith('_duration'))) {
    return false;
  }
  return true;
}

/**
 * Sanitizes a label key to ensure it matches ^[a-zA-Z_][a-zA-Z0-9_]*$.
 */
export function sanitizeLabelKey(key: string): string {
  if (!key) return 'unknown';
  let sanitized = key.replace(/[^a-zA-Z0-9_]/g, '_');
  if (/^[0-9]/.test(sanitized)) {
    sanitized = '_' + sanitized;
  }
  return sanitized || 'unknown';
}

/**
 * Sanitizes and truncates a label value to prevent cardinality explosion.
 */
export function sanitizeLabelValue(val: unknown, maxLength = 64): string {
  if (val === null || val === undefined) {
    return 'unknown';
  }
  const str = String(val).trim();
  if (!str) {
    return 'unknown';
  }
  if (str.length > maxLength) {
    return str.substring(0, maxLength) + '...';
  }
  return str;
}

/**
 * Registry holding counters, gauges, and histograms.
 */
export class BusinessMetricsRegistry {
  private prefix: string;
  private maxCardinality: number;
  private maxLabelLength: number;

  private definitions = new Map<string, MetricDefinition>();
  private counters = new Map<string, Map<string, number>>();
  private gauges = new Map<string, Map<string, number>>();
  private histograms = new Map<string, Map<string, { sum: number; count: number; bucketCounts: number[] }>>();

  constructor(options: MetricsRegistryOptions = {}) {
    this.prefix = options.prefix ? options.prefix.replace(/[^a-zA-Z0-9_]/g, '_') + '_' : '';
    this.maxCardinality = options.maxCardinalityPerMetric ?? 500;
    this.maxLabelLength = options.maxLabelValueLength ?? 64;
  }

  /**
   * Registers a metric definition.
   */
  public register(def: MetricDefinition): void {
    const fullName = this.formatName(def.name);
    if (!validateMetricName(fullName, def.type)) {
      throw new Error(`Invalid metric name '${fullName}' for type '${def.type}'. Must follow naming conventions.`);
    }
    this.definitions.set(fullName, {
      ...def,
      name: fullName,
      buckets: def.buckets ?? (def.type === 'histogram' ? DEFAULT_HISTOGRAM_BUCKETS : undefined),
    });
  }

  /**
   * Increments a counter metric.
   */
  public incrementCounter(name: string, value = 1, labels: MetricLabels = {}): void {
    const fullName = this.formatName(name);
    const def = this.definitions.get(fullName);
    if (!def || def.type !== 'counter') {
      this.register({ name: name, help: `Counter for ${name}`, type: 'counter' });
    }

    if (isNaN(value) || !isFinite(value) || value < 0) {
      value = 0; // Ignore non-numeric or negative counter increments
    }

    const metricMap = this.getOrCreateMetricMap(this.counters, fullName);
    const labelKey = this.serializeLabels(labels);

    if (!metricMap.has(labelKey) && metricMap.size >= this.maxCardinality) {
      const overflowKey = 'overflow=true';
      metricMap.set(overflowKey, (metricMap.get(overflowKey) ?? 0) + value);
      return;
    }

    metricMap.set(labelKey, (metricMap.get(labelKey) ?? 0) + value);
  }

  /**
   * Sets a gauge metric value.
   */
  public setGauge(name: string, value: number, labels: MetricLabels = {}): void {
    const fullName = this.formatName(name);
    const def = this.definitions.get(fullName);
    if (!def || def.type !== 'gauge') {
      this.register({ name: name, help: `Gauge for ${name}`, type: 'gauge' });
    }

    if (isNaN(value) || !isFinite(value)) {
      return;
    }

    const metricMap = this.getOrCreateMetricMap(this.gauges, fullName);
    const labelKey = this.serializeLabels(labels);

    if (!metricMap.has(labelKey) && metricMap.size >= this.maxCardinality) {
      return;
    }

    metricMap.set(labelKey, value);
  }

  /**
   * Records a value in a histogram metric.
   */
  public recordHistogram(name: string, value: number, labels: MetricLabels = {}): void {
    const fullName = this.formatName(name);
    let def = this.definitions.get(fullName);
    if (!def || def.type !== 'histogram') {
      this.register({ name: name, help: `Histogram for ${name}`, type: 'histogram', buckets: DEFAULT_HISTOGRAM_BUCKETS });
      def = this.definitions.get(fullName)!;
    }

    if (isNaN(value) || !isFinite(value) || value < 0) {
      return;
    }

    const metricMap = this.getOrCreateMetricMap(this.histograms, fullName);
    const labelKey = this.serializeLabels(labels);
    const buckets = def.buckets ?? DEFAULT_HISTOGRAM_BUCKETS;

    let entry = metricMap.get(labelKey);
    if (!entry) {
      if (metricMap.size >= this.maxCardinality) {
        return;
      }
      entry = { sum: 0, count: 0, bucketCounts: new Array(buckets.length).fill(0) };
      metricMap.set(labelKey, entry);
    }

    entry.sum += value;
    entry.count += 1;
    for (let i = 0; i < buckets.length; i++) {
      if (value <= buckets[i]) {
        entry.bucketCounts[i] += 1;
      }
    }
  }

  /**
   * Exports all recorded metrics in structured format.
   */
  public getMetrics(): { counters: MetricValue[]; gauges: MetricValue[]; histograms: HistogramValue[] } {
    const now = Date.now();
    const counters: MetricValue[] = [];
    const gauges: MetricValue[] = [];
    const histograms: HistogramValue[] = [];

    // Export counters
    for (const [name, map] of this.counters.entries()) {
      for (const [labelStr, val] of map.entries()) {
        counters.push({
          name,
          type: 'counter',
          value: val,
          labels: this.deserializeLabels(labelStr),
          timestamp: now,
        });
      }
    }

    // Export gauges
    for (const [name, map] of this.gauges.entries()) {
      for (const [labelStr, val] of map.entries()) {
        gauges.push({
          name,
          type: 'gauge',
          value: val,
          labels: this.deserializeLabels(labelStr),
          timestamp: now,
        });
      }
    }

    // Export histograms
    for (const [name, map] of this.histograms.entries()) {
      const def = this.definitions.get(name);
      const buckets = def?.buckets ?? DEFAULT_HISTOGRAM_BUCKETS;

      for (const [labelStr, entry] of map.entries()) {
        const bucketList: HistogramBucket[] = buckets.map((le, idx) => ({
          le,
          count: entry.bucketCounts[idx],
        }));
        bucketList.push({ le: Infinity, count: entry.count });

        histograms.push({
          name,
          sum: entry.sum,
          count: entry.count,
          buckets: bucketList,
          labels: this.deserializeLabels(labelStr),
          timestamp: now,
        });
      }
    }

    return { counters, gauges, histograms };
  }

  /**
   * Resets all metric stores.
   */
  public reset(): void {
    this.counters.clear();
    this.gauges.clear();
    this.histograms.clear();
  }

  private formatName(name: string): string {
    if (this.prefix && !name.startsWith(this.prefix)) {
      return this.prefix + name;
    }
    return name;
  }

  private getOrCreateMetricMap<T>(store: Map<string, Map<string, T>>, name: string): Map<string, T> {
    let map = store.get(name);
    if (!map) {
      map = new Map<string, T>();
      store.set(name, map);
    }
    return map;
  }

  private serializeLabels(labels: MetricLabels): string {
    const keys = Object.keys(labels).sort();
    if (keys.length === 0) return '';

    const pairs: string[] = [];
    for (const rawKey of keys) {
      const key = sanitizeLabelKey(rawKey);
      const val = sanitizeLabelValue(labels[rawKey], this.maxLabelLength);
      pairs.push(`${key}="${val}"`);
    }
    return pairs.join(',');
  }

  private deserializeLabels(str: string): Record<string, string> {
    if (!str) return {};
    if (str === 'overflow=true') return { overflow: 'true' };

    const result: Record<string, string> = {};
    const parts = str.split(',');
    for (const part of parts) {
      const eqIdx = part.indexOf('=');
      if (eqIdx !== -1) {
        const key = part.substring(0, eqIdx);
        let val = part.substring(eqIdx + 1);
        if (val.startsWith('"') && val.endsWith('"')) {
          val = val.substring(1, val.length - 1);
        }
        result[key] = val;
      }
    }
    return result;
  }
}

/**
 * Standard Singleton Business Metrics Instance for Production Visibility
 */
export const defaultBusinessRegistry = new BusinessMetricsRegistry({ prefix: 'grainlify' });

// Pre-register standard application domain metrics
defaultBusinessRegistry.register({ name: 'grainlify_bounty_created_total', help: 'Total bounties created', type: 'counter' });
defaultBusinessRegistry.register({ name: 'grainlify_bounty_settled_total', help: 'Total bounties settled', type: 'counter' });
defaultBusinessRegistry.register({ name: 'grainlify_escrow_locked_bytes', help: 'Total escrow funds locked in bytes/subunits', type: 'counter' });
defaultBusinessRegistry.register({ name: 'grainlify_http_request_duration_seconds', help: 'HTTP request duration in seconds', type: 'histogram' });
defaultBusinessRegistry.register({ name: 'grainlify_active_users_gauge', help: 'Current active users count', type: 'gauge' });
defaultBusinessRegistry.register({ name: 'grainlify_rate_limit_exceeded_total', help: 'Total rate limit exceeded events', type: 'counter' });
