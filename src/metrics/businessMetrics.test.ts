import {
  BusinessMetricsRegistry,
  validateMetricName,
  sanitizeLabelKey,
  sanitizeLabelValue,
  defaultBusinessRegistry
} from './businessMetrics';

describe('Business Monitoring Metrics System', () => {
  describe('Naming Conventions and Sanitization', () => {
    it('validates counter naming rules (_total suffix)', () => {
      expect(validateMetricName('grainlify_bounty_created_total', 'counter')).toBe(true);
      expect(validateMetricName('grainlify_bounty_created', 'counter')).toBe(false);
      expect(validateMetricName('123_invalid_name_total', 'counter')).toBe(false);
    });

    it('validates histogram naming rules (_seconds, _ms, _bytes, _duration suffixes)', () => {
      expect(validateMetricName('grainlify_http_request_duration_seconds', 'histogram')).toBe(true);
      expect(validateMetricName('grainlify_payload_size_bytes', 'histogram')).toBe(true);
      expect(validateMetricName('grainlify_http_request_latency', 'histogram')).toBe(false);
    });

    it('validates gauge naming rules', () => {
      expect(validateMetricName('grainlify_active_users_count', 'gauge')).toBe(true);
      expect(validateMetricName('grainlify_memory_usage_ratio', 'gauge')).toBe(true);
    });

    it('sanitizes non-alphanumeric label keys', () => {
      expect(sanitizeLabelKey('user-id')).toBe('user_id');
      expect(sanitizeLabelKey('http.status.code')).toBe('http_status_code');
      expect(sanitizeLabelKey('123key')).toBe('_123key');
      expect(sanitizeLabelKey('')).toBe('unknown');
    });

    it('sanitizes and truncates label values to prevent high cardinality', () => {
      expect(sanitizeLabelValue('production')).toBe('production');
      expect(sanitizeLabelValue(null)).toBe('unknown');
      expect(sanitizeLabelValue(undefined)).toBe('unknown');

      const longString = 'a'.repeat(100);
      const truncated = sanitizeLabelValue(longString, 10);
      expect(truncated).toBe('aaaaaaaaaa...');
    });
  });

  describe('BusinessMetricsRegistry Functional Tests', () => {
    let registry: BusinessMetricsRegistry;

    beforeEach(() => {
      registry = new BusinessMetricsRegistry({ prefix: 'app', maxCardinalityPerMetric: 5 });
    });

    it('records and exports counter metrics correctly', () => {
      registry.incrementCounter('bounty_created_total', 1, { ecosystem: 'stellar', tier: 'gold' });
      registry.incrementCounter('bounty_created_total', 2, { ecosystem: 'stellar', tier: 'gold' });

      const exported = registry.getMetrics();
      expect(exported.counters).toHaveLength(1);
      expect(exported.counters[0].name).toBe('app_bounty_created_total');
      expect(exported.counters[0].value).toBe(3);
      expect(exported.counters[0].labels).toEqual({ ecosystem: 'stellar', tier: 'gold' });
    });

    it('records and exports gauge metrics correctly', () => {
      registry.setGauge('active_users', 42, { environment: 'production' });
      registry.setGauge('active_users', 50, { environment: 'production' });

      const exported = registry.getMetrics();
      expect(exported.gauges).toHaveLength(1);
      expect(exported.gauges[0].name).toBe('app_active_users');
      expect(exported.gauges[0].value).toBe(50);
      expect(exported.gauges[0].labels).toEqual({ environment: 'production' });
    });

    it('records histogram latency distributions into correct buckets', () => {
      registry.recordHistogram('request_duration_seconds', 0.02, { path: '/api/v1/bounty' });
      registry.recordHistogram('request_duration_seconds', 0.4, { path: '/api/v1/bounty' });

      const exported = registry.getMetrics();
      expect(exported.histograms).toHaveLength(1);
      const hist = exported.histograms[0];
      expect(hist.name).toBe('app_request_duration_seconds');
      expect(hist.count).toBe(2);
      expect(hist.sum).toBeCloseTo(0.42);
      expect(hist.labels).toEqual({ path: '/api/v1/bounty' });
    });

    it('enforces maximum cardinality per metric with overflow protection', () => {
      // Record 6 unique label pairs when max cardinality is 5
      for (let i = 0; i < 6; i++) {
        registry.incrementCounter('bounty_created_total', 1, { id: `id_${i}` });
      }

      const exported = registry.getMetrics();
      // Should cap unique items and route overflow into overflow bucket
      expect(exported.counters.length).toBeLessThanOrEqual(6);
      const overflow = exported.counters.find(c => c.labels.overflow === 'true');
      expect(overflow).toBeDefined();
      expect(overflow?.value).toBe(1);
    });

    it('handles NaN, Infinity, negative counter increments gracefully', () => {
      registry.incrementCounter('bounty_created_total', NaN);
      registry.incrementCounter('bounty_created_total', Infinity);
      registry.incrementCounter('bounty_created_total', -10);

      const exported = registry.getMetrics();
      expect(exported.counters[0].value).toBe(0);
    });

    it('resets registry state cleanly', () => {
      registry.incrementCounter('bounty_created_total', 5);
      registry.reset();

      const exported = registry.getMetrics();
      expect(exported.counters).toHaveLength(0);
      expect(exported.gauges).toHaveLength(0);
      expect(exported.histograms).toHaveLength(0);
    });
  });

  describe('Default Application Instance Integration', () => {
    it('has pre-registered standard domain metrics', () => {
      defaultBusinessRegistry.incrementCounter('grainlify_bounty_created_total', 1, { ecosystem: 'soroban' });
      const exported = defaultBusinessRegistry.getMetrics();
      expect(exported.counters.some(c => c.name === 'grainlify_bounty_created_total')).toBe(true);
    });
  });
});
