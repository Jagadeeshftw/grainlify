/**
 * pagination-helpers.test.ts
 *
 * Tests for the `fetchAllPages` pagination helper and the
 * `queryPayoutHistory` / `queryReleaseSchedules` query methods added to
 * `ProgramEscrowClient` (Issue #44 – history/query SDK exposure).
 *
 * All tests mock `invokeContract` (via `(client as any).invokeContract`)
 * to avoid real network calls — same pattern used in network-errors.test.ts.
 */

import { ProgramEscrowClient, fetchAllPages } from "../program-escrow-client";
import type {
  PayoutRecord,
  ProgramReleaseSchedule,
  PayoutQueryFilter,
  ScheduleQueryFilter,
} from "../program-escrow-client";

// ---------------------------------------------------------------------------
// Test data factories
// ---------------------------------------------------------------------------

function makePayoutRecord(index: number): PayoutRecord {
  return {
    recipient: `G${"A".repeat(54)}${String(index).padStart(2, "0")}`,
    amount: BigInt(index * 1_000),
    timestamp: 1_700_000_000 + index,
  };
}

function makeSchedule(index: number): ProgramReleaseSchedule {
  return {
    schedule_id: BigInt(index),
    recipient: `G${"A".repeat(54)}${String(index).padStart(2, "0")}`,
    amount: BigInt(index * 500),
    release_timestamp: 1_700_000_000 + index * 3_600,
    released: false,
  };
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function makeClient(): ProgramEscrowClient {
  return new ProgramEscrowClient({
    contractId:
      "CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC",
    rpcUrl: "https://soroban-testnet.stellar.org",
    networkPassphrase: "Test SDF Network ; September 2015",
  });
}

// ---------------------------------------------------------------------------
// fetchAllPages — standalone helper
// ---------------------------------------------------------------------------

describe("fetchAllPages", () => {
  // ── Empty dataset ──────────────────────────────────────────────────────────
  describe("empty dataset", () => {
    it("returns [] when the first page is already empty", async () => {
      const fetcher = jest.fn().mockResolvedValue([]);
      const result = await fetchAllPages(fetcher, 10);

      expect(result).toEqual([]);
      // Only one call: offset 0; empty response stops immediately.
      expect(fetcher).toHaveBeenCalledTimes(1);
      expect(fetcher).toHaveBeenCalledWith(0, 10);
    });
  });

  // ── Single page ────────────────────────────────────────────────────────────
  describe("single page — fewer items than pageSize", () => {
    it("returns items when all data fits in one page", async () => {
      const items = [makePayoutRecord(1), makePayoutRecord(2)];
      const fetcher = jest
        .fn()
        .mockResolvedValueOnce(items)
        .mockResolvedValue([]); // should not be reached in this scenario
      const result = await fetchAllPages(fetcher, 10);

      expect(result).toEqual(items);
      // Partial first page -> stop; no call at offset 10.
      expect(fetcher).toHaveBeenCalledTimes(1);
    });
  });

  // ── Full first page → empty second page ───────────────────────────────────
  describe("single full page followed by empty terminator", () => {
    it("calls fetcher twice and returns the correct items", async () => {
      const PAGE_SIZE = 3;
      const page0 = Array.from({ length: 3 }, (_, i) => makePayoutRecord(i));
      const fetcher = jest
        .fn()
        .mockResolvedValueOnce(page0)
        .mockResolvedValue([]);

      const result = await fetchAllPages(fetcher, PAGE_SIZE);

      expect(result).toHaveLength(3);
      expect(fetcher).toHaveBeenCalledTimes(2);
      expect(fetcher).toHaveBeenNthCalledWith(1, 0, PAGE_SIZE);
      expect(fetcher).toHaveBeenNthCalledWith(2, 3, PAGE_SIZE);
    });
  });

  // ── Multiple pages ─────────────────────────────────────────────────────────
  describe("multiple pages", () => {
    it("concatenates full pages plus a partial last page", async () => {
      const PAGE_SIZE = 5;
      const page0 = Array.from({ length: 5 }, (_, i) => makePayoutRecord(i));
      const page1 = Array.from({ length: 5 }, (_, i) =>
        makePayoutRecord(i + 5),
      );
      const page2 = Array.from({ length: 2 }, (_, i) =>
        makePayoutRecord(i + 10),
      );

      const fetcher = jest
        .fn()
        .mockResolvedValueOnce(page0)
        .mockResolvedValueOnce(page1)
        .mockResolvedValueOnce(page2);

      const result = await fetchAllPages(fetcher, PAGE_SIZE);
      expect(result).toHaveLength(12);
      expect(result).toEqual([...page0, ...page1, ...page2]);
    });

    it("increments offset by pageSize on every call", async () => {
      const PAGE_SIZE = 4;
      const fetcher = jest
        .fn()
        .mockResolvedValueOnce(
          Array.from({ length: 4 }, (_, i) => makePayoutRecord(i)),
        )
        .mockResolvedValueOnce(
          Array.from({ length: 4 }, (_, i) => makePayoutRecord(i + 4)),
        )
        .mockResolvedValueOnce([makePayoutRecord(8)]); // partial last page

      await fetchAllPages(fetcher, PAGE_SIZE);

      expect(fetcher).toHaveBeenCalledTimes(3);
      expect(fetcher).toHaveBeenNthCalledWith(1, 0, PAGE_SIZE);
      expect(fetcher).toHaveBeenNthCalledWith(2, 4, PAGE_SIZE);
      expect(fetcher).toHaveBeenNthCalledWith(3, 8, PAGE_SIZE);
    });

    it("handles a large dataset (10 pages of 50) correctly", async () => {
      const PAGE_SIZE = 50;
      const fetcher = jest.fn().mockImplementation((offset: number) => {
        if (offset < 500) {
          return Promise.resolve(
            Array.from({ length: PAGE_SIZE }, (_, i) =>
              makePayoutRecord(offset + i),
            ),
          );
        }
        return Promise.resolve([]);
      });

      const result = await fetchAllPages<PayoutRecord>(fetcher, PAGE_SIZE);

      expect(result).toHaveLength(500);
      // No duplicates — every timestamp must be unique.
      expect(new Set(result.map((r) => r.timestamp)).size).toBe(500);
    });
  });

  // ── Edge: pageSize = 1 ─────────────────────────────────────────────────────
  describe("edge case: pageSize = 1", () => {
    it("paginates correctly with single-item pages", async () => {
      const items = [
        makePayoutRecord(0),
        makePayoutRecord(1),
        makePayoutRecord(2),
      ];
      const fetcher = jest
        .fn()
        .mockImplementation((offset: number) =>
          Promise.resolve(offset < items.length ? [items[offset]] : []),
        );

      const result = await fetchAllPages(fetcher, 1);
      expect(result).toEqual(items);
      // 3 data pages + 1 empty terminator
      expect(fetcher).toHaveBeenCalledTimes(4);
    });
  });

  // ── Default pageSize = 50 ─────────────────────────────────────────────────
  describe("default pageSize", () => {
    it("uses 50 as the default page size when none is specified", async () => {
      const fetcher = jest.fn().mockResolvedValue([]);
      await fetchAllPages(fetcher);
      expect(fetcher).toHaveBeenCalledWith(0, 50);
    });
  });

  // ── Exact-once invariant ───────────────────────────────────────────────────
  describe("exact-once retrieval invariant", () => {
    it("retrieves each item exactly once, in order, with no duplicates", async () => {
      const PAGE_SIZE = 3;
      const allItems = Array.from({ length: 7 }, (_, i) => makePayoutRecord(i));
      const fetcher = jest
        .fn()
        .mockImplementation((offset: number) =>
          Promise.resolve(allItems.slice(offset, offset + PAGE_SIZE)),
        );

      const result = await fetchAllPages<PayoutRecord>(fetcher, PAGE_SIZE);

      expect(result).toHaveLength(7);
      expect(result).toEqual(allItems);
      // Every timestamp is unique — no item was returned twice.
      expect(new Set(result.map((r) => r.timestamp)).size).toBe(7);
    });
  });

  // ── Async / latency correctness ────────────────────────────────────────────
  describe("async behavior", () => {
    it("collects all items in order even when responses are delayed", async () => {
      const PAGE_SIZE = 3;
      const allItems = Array.from({ length: 6 }, (_, i) => makePayoutRecord(i));

      const fetcher = jest.fn().mockImplementation(async (offset: number) => {
        await delay(5);
        return allItems.slice(offset, offset + PAGE_SIZE);
      });

      const result = await fetchAllPages(fetcher, PAGE_SIZE);
      expect(result).toEqual(allItems);
    });

    it("fetches pages sequentially — not concurrently", async () => {
      const PAGE_SIZE = 2;
      const callOrder: number[] = [];

      const fetcher = jest.fn().mockImplementation(async (offset: number) => {
        callOrder.push(offset);
        await delay(5);
        // Two full pages (offsets 0, 2) then empty at offset 4.
        if (offset < 4) {
          return [makePayoutRecord(offset), makePayoutRecord(offset + 1)];
        }
        return [];
      });

      await fetchAllPages(fetcher, PAGE_SIZE);

      // Offsets must appear in strictly ascending order, proving no
      // concurrent calls were started before prior pages resolved.
      expect(callOrder).toEqual([0, 2, 4]);
    });
  });
});

// ---------------------------------------------------------------------------
// ProgramEscrowClient.queryPayoutHistory
// ---------------------------------------------------------------------------

describe("ProgramEscrowClient.queryPayoutHistory", () => {
  let client: ProgramEscrowClient;

  beforeEach(() => {
    client = makeClient();
  });

  it("returns [] when the contract returns no records", async () => {
    (client as any).invokeContract = jest.fn().mockResolvedValue([]);
    const result = await client.queryPayoutHistory({}, 0, 10);
    expect(result).toEqual([]);
  });

  it("returns payout records for a single page", async () => {
    const records = [makePayoutRecord(1), makePayoutRecord(2)];
    (client as any).invokeContract = jest.fn().mockResolvedValue(records);
    const result = await client.queryPayoutHistory({}, 0, 10);
    expect(result).toEqual(records);
  });

  it("forwards the correct method name, offset, and limit to invokeContract", async () => {
    const mockInvoke = jest.fn().mockResolvedValue([]);
    (client as any).invokeContract = mockInvoke;

    await client.queryPayoutHistory({}, 20, 5);

    const [method, args] = mockInvoke.mock.calls[0];
    expect(method).toBe("query_payout_history");
    expect(args[1]).toBe(20); // offset
    expect(args[2]).toBe(5); // limit
  });

  it("shallow-copies the filter -- original object is not mutated", async () => {
    const mockInvoke = jest.fn().mockResolvedValue([]);
    (client as any).invokeContract = mockInvoke;

    const filter: PayoutQueryFilter = {
      recipient: "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
      min_amount: 100n,
    };
    const originalRecipient = filter.recipient;
    const originalMinAmount = filter.min_amount;

    await client.queryPayoutHistory(filter, 0, 10);

    // Caller's filter must be unchanged.
    expect(filter.recipient).toBe(originalRecipient);
    expect(filter.min_amount).toBe(originalMinAmount);

    // The object forwarded to invokeContract must be a different reference…
    const [, args] = mockInvoke.mock.calls[0];
    expect(args[0]).not.toBe(filter);
    // …but with equal contents.
    expect(args[0]).toEqual(filter);
  });

  it("propagates errors through handleError", async () => {
    (client as any).invokeContract = jest
      .fn()
      .mockRejectedValue(
        Object.assign(new Error("RPC error"), { response: { status: 503 } }),
      );
    await expect(client.queryPayoutHistory({}, 0, 10)).rejects.toThrow();
  });

  // ── Integration with fetchAllPages ────────────────────────────────────────
  describe("integration with fetchAllPages", () => {
    it("retrieves all payout records across multiple pages with no duplicates", async () => {
      const allRecords = Array.from({ length: 7 }, (_, i) =>
        makePayoutRecord(i),
      );
      const PAGE_SIZE = 3;

      (client as any).invokeContract = jest
        .fn()
        .mockImplementation((_method: string, args: any[]) =>
          Promise.resolve(allRecords.slice(args[1], args[1] + args[2])),
        );

      const result = await fetchAllPages(
        (offset, limit) => client.queryPayoutHistory({}, offset, limit),
        PAGE_SIZE,
      );

      expect(result).toHaveLength(allRecords.length);
      expect(result).toEqual(allRecords);
      expect(new Set(result.map((r) => r.timestamp)).size).toBe(
        allRecords.length,
      );
    });
  });
});

// ---------------------------------------------------------------------------
// ProgramEscrowClient.queryReleaseSchedules
// ---------------------------------------------------------------------------

describe("ProgramEscrowClient.queryReleaseSchedules", () => {
  let client: ProgramEscrowClient;

  beforeEach(() => {
    client = makeClient();
  });

  it("returns [] when there are no schedules", async () => {
    (client as any).invokeContract = jest.fn().mockResolvedValue([]);
    const result = await client.queryReleaseSchedules({}, 0, 10);
    expect(result).toEqual([]);
  });

  it("returns pending schedules from the contract", async () => {
    const schedules = [makeSchedule(1), makeSchedule(2), makeSchedule(3)];
    (client as any).invokeContract = jest.fn().mockResolvedValue(schedules);
    const result = await client.queryReleaseSchedules(
      { released: false },
      0,
      10,
    );
    expect(result).toEqual(schedules);
  });

  it("forwards the correct method name, offset, and limit to invokeContract", async () => {
    const mockInvoke = jest.fn().mockResolvedValue([]);
    (client as any).invokeContract = mockInvoke;

    await client.queryReleaseSchedules({}, 15, 25);

    const [method, args] = mockInvoke.mock.calls[0];
    expect(method).toBe("query_release_schedules");
    expect(args[1]).toBe(15);
    expect(args[2]).toBe(25);
  });

  it("shallow-copies the filter -- original object is not mutated", async () => {
    const mockInvoke = jest.fn().mockResolvedValue([]);
    (client as any).invokeContract = mockInvoke;

    const filter: ScheduleQueryFilter = { released: false, min_amount: 500n };
    const originalReleased = filter.released;
    const originalMinAmount = filter.min_amount;

    await client.queryReleaseSchedules(filter, 0, 50);

    expect(filter.released).toBe(originalReleased);
    expect(filter.min_amount).toBe(originalMinAmount);

    const [, args] = mockInvoke.mock.calls[0];
    expect(args[0]).not.toBe(filter);
    expect(args[0]).toEqual(filter);
  });

  it("propagates errors through handleError", async () => {
    (client as any).invokeContract = jest
      .fn()
      .mockRejectedValue(
        Object.assign(new Error("timeout"), { code: "ETIMEDOUT" }),
      );
    await expect(client.queryReleaseSchedules({}, 0, 10)).rejects.toThrow();
  });

  // ── Integration with fetchAllPages ────────────────────────────────────────
  describe("integration with fetchAllPages", () => {
    it("retrieves all schedules across multiple pages with no duplicates", async () => {
      const allSchedules = Array.from({ length: 11 }, (_, i) =>
        makeSchedule(i),
      );
      const PAGE_SIZE = 4;

      (client as any).invokeContract = jest
        .fn()
        .mockImplementation((_method: string, args: any[]) =>
          Promise.resolve(allSchedules.slice(args[1], args[1] + args[2])),
        );

      const result = await fetchAllPages(
        (offset, limit) => client.queryReleaseSchedules({}, offset, limit),
        PAGE_SIZE,
      );

      expect(result).toHaveLength(11);
      expect(result).toEqual(allSchedules);
      // No duplicate schedule IDs.
      expect(new Set(result.map((s) => String(s.schedule_id))).size).toBe(11);
    });
  });
});
