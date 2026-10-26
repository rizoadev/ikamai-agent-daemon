/**
 * k6 load test for IAD REST API (Phase 3.15).
 * Run: k6 run tests/load/k6_api.js --env BASE_URL=http://127.0.0.1:7700
 */
import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

const errorRate = new Rate('errors');
const agentLat  = new Trend('agent_list_latency');
const submitLat = new Trend('task_submit_latency');

export const options = {
  stages: [
    { duration: '10s', target: 10  },
    { duration: '30s', target: 50  },
    { duration: '20s', target: 100 },
    { duration: '10s', target: 0   },
  ],
  thresholds: {
    http_req_duration: ['p(95)<200'],
    errors:            ['rate<0.01'],
  },
};

const BASE  = __ENV.BASE_URL  || 'http://127.0.0.1:7700';
const TOKEN = __ENV.API_TOKEN || '';
const HDR   = TOKEN ? { Authorization: `Bearer ${TOKEN}` } : {};

export default function () {
  const r1 = http.get(`${BASE}/healthz`);
  check(r1, { 'healthz 200': (r) => r.status === 200 });
  errorRate.add(r1.status !== 200);

  const t0 = Date.now();
  const r2 = http.get(`${BASE}/api/v1/agents`, { headers: HDR });
  agentLat.add(Date.now() - t0);
  check(r2, { 'agents 200': (r) => r.status === 200 });
  errorRate.add(r2.status !== 200);

  const t1 = Date.now();
  const r3 = http.post(
    `${BASE}/api/v1/tasks`,
    JSON.stringify({ payload: { text: 'k6', capability: 'echo' }, capabilities: ['echo'], priority: 3 }),
    { headers: { ...HDR, 'Content-Type': 'application/json' } }
  );
  submitLat.add(Date.now() - t1);
  check(r3, { 'submit 200': (r) => r.status === 200 });
  errorRate.add(r3.status !== 200);

  const r4 = http.get(`${BASE}/metrics`);
  check(r4, { 'metrics 200': (r) => r.status === 200 });

  sleep(0.1);
}
