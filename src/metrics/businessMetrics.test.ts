import {
  BusinessMetricsRegistry,
  validateMetricName,
  sanitizeLabelKey,
  sanitizeLabelValue,
  defaultBusinessRegistry,
  resetDefaultRegistry
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

  describe('Deterministic Output and Edge Cases', () => {
    let registry: BusinessMetricsRegistry;

    beforeEach(() => {
      registry = new BusinessMetricsRegistry({ prefix: 'app', maxCardinalityPerMetric: 5 });
    });

    it('exports metrics in deterministic order (sorted by name and labels)', () => {
      // Record metrics in non-alphabetical order
      registry.incrementCounter('zebra_total', 1, { label: 'c' });
      registry.incrementCounter('alpha_total', 1, { label: 'a' });
      registry.incrementCounter('beta_total', 1, { label: 'b' });

      const exported = registry.getMetrics();
      expect(exported.counters[0].name).toBe('app_alpha_total');
      expect(exported.counters[1].name).toBe('app_beta_total');
      expect(exported.counters[2].name).toBe('app_zebra_total');
    });

    it('exports label combinations in deterministic order', () => {
      registry.incrementCounter('test_total', 1, { z: '1', a: '2' });
      registry.incrementCounter('test_total', 1, { m: '3' });

      const exported = registry.getMetrics();
      // Labels are sorted by key during serialization
      expect(exported.counters[0].labels).toEqual({ a: '2', z: '1' });
      expect(exported.counters[1].labels).toEqual({ m: '3' });
    });

    it('auto-registers metrics with default help text when not pre-registered', () => {
      registry.incrementCounter('custom_metric_total', 5);
      const exported = registry.getMetrics();
      expect(exported.counters[0].name).toBe('app_custom_metric_total');
      expect(exported.counters[0].value).toBe(5);
    });

    it('handles prefix duplication - calling with full prefixed name does not double-prefix', () => {
      registry.incrementCounter('app_test_total', 1);
      const exported = registry.getMetrics();
      expect(exported.counters[0].name).toBe('app_test_total');
      expect(exported.counters[0].name).not.toBe('app_app_test_total');
    });

    it('truncates long label values with ellipsis suffix', () => {
      registry.incrementCounter('test_total', 1, { long: 'x'.repeat(100) });
      const exported = registry.getMetrics();
      expect(exported.counters[0].labels.long).toBe('x'.repeat(64) + '...');
    });

    it('uses hardcoded overflow key format for cardinality protection', () => {
      // Record more unique label combinations than maxCardinality
      for (let i = 0; i < 6; i++) {
        registry.incrementCounter('test_total', 1, { id: `unique_${i}` });
      }

      const exported = registry.getMetrics();
      const overflowMetric = exported.counters.find(c => c.labels.overflow === 'true');
      expect(overflowMetric).toBeDefined();
      expect(overflowMetric?.labels).toEqual({ overflow: 'true' });
    });

    it('handles empty label sets consistently', () => {
      registry.incrementCounter('test_total', 1, {});
      registry.incrementCounter('test_total', 1);

      const exported = registry.getMetrics();
      // Both should result in the same label key (empty)
      expect(exported.counters).toHaveLength(1);
      expect(exported.counters[0].value).toBe(2);
      expect(exported.counters[0].labels).toEqual({});
    });

    it('handles special characters in label keys deterministically', () => {
      registry.incrementCounter('test_total', 1, { 'user-id': '123', 'http.status': '200' });
      const exported = registry.getMetrics();
      expect(exported.counters[0].labels).toEqual({ 'user_id': '123', 'http_status': '200' });
    });

    it('handles numeric-first label keys by prefixing with underscore', () => {
      registry.incrementCounter('test_total', 1, { '123key': 'value' });
      const exported = registry.getMetrics();
      expect(exported.counters[0].labels).toEqual({ _123key: 'value' });
    });

    it('handles null and undefined label values as "unknown"', () => {
      registry.incrementCounter('test_total', 1, { null_val: null, undef_val: undefined });
      const exported = registry.getMetrics();
      expect(exported.counters[0].labels.null_val).toBe('unknown');
      expect(exported.counters[0].labels.undef_val).toBe('unknown');
    });

    it('handles empty string label values as "unknown"', () => {
      registry.incrementCounter('test_total', 1, { empty: '' });
      const exported = registry.getMetrics();
      expect(exported.counters[0].labels.empty).toBe('unknown');
    });

    it('resets default registry in test environment', () => {
      // Set NODE_ENV to test for this test
      const originalEnv = process.env.NODE_ENV;
      process.env.NODE_ENV = 'test';

      defaultBusinessRegistry.incrementCounter('grainlify_bounty_created_total', 10);
      resetDefaultRegistry();
      const exported = defaultBusinessRegistry.getMetrics();
      expect(exported.counters.length).toBe(0);

      process.env.NODE_ENV = originalEnv;
    });

    it('does not reset default registry in non-test environment', () => {
      const originalEnv = process.env.NODE_ENV;
      process.env.NODE_ENV = 'production';

      defaultBusinessRegistry.incrementCounter('grainlify_bounty_created_total', 10);
      resetDefaultRegistry();
      const exported = defaultBusinessRegistry.getMetrics();
      expect(exported.counters.length).toBeGreaterThan(0);

      process.env.NODE_ENV = originalEnv;
    });
  });
});
