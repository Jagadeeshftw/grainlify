import { Address, nativeToScVal, xdr } from "@stellar/stellar-sdk";
import { ValidationError } from "./errors";
import programEscrowSpecJson from "./specs/program-escrow-spec.json";

export interface SpecParameter {
  name: string;
  type: string;
}

export interface SpecEntrypoint {
  name: string;
  description?: string;
  readOnly: boolean;
  parameters: SpecParameter[];
}

export interface SpecStructDefinition {
  kind: "struct";
  fields: SpecParameter[];
}

export interface SpecEnumVariant {
  name: string;
  fields?: string[];
}

export interface SpecEnumDefinition {
  kind: "enum";
  variants: SpecEnumVariant[];
}

export interface SpecU32EnumDefinition {
  kind: "u32Enum";
  variants: Record<string, number>;
}

export type SpecTypeDefinition =
  | SpecStructDefinition
  | SpecEnumDefinition
  | SpecU32EnumDefinition;

export interface ContractSpecDefinition {
  contractName: string;
  version?: string;
  types?: Record<string, SpecTypeDefinition>;
  entrypoints: SpecEntrypoint[];
}

export interface ManifestEntrypoint {
  name: string;
  description?: string;
  parameters?: SpecParameter[];
  authorization?: string;
  readOnly?: boolean;
}

export interface ContractManifest {
  contract_name: string;
  version?: { current?: string };
  entrypoints: {
    public?: ManifestEntrypoint[];
    admin?: ManifestEntrypoint[];
    view?: ManifestEntrypoint[];
  };
}

/**
 * Splits a top-level comma-separated generic type argument list,
 * respecting nested angle brackets (e.g. `Map<String, Vec<Option<i128>>>`).
 */
function splitTopLevelTypeArgs(inner: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let start = 0;

  for (let i = 0; i < inner.length; i++) {
    const ch = inner[i];
    if (ch === "<") {
      depth++;
    } else if (ch === ">") {
      depth--;
    } else if (ch === "," && depth === 0) {
      parts.push(inner.slice(start, i).trim());
      start = i + 1;
    }
  }
  const tail = inner.slice(start).trim();
  if (tail.length > 0) {
    parts.push(tail);
  }
  return parts;
}

function isOptionType(typeExpr: string): boolean {
  const trimmed = typeExpr.trim();
  return trimmed.startsWith("Option<") && trimmed.endsWith(">");
}

function unwrapGeneric(typeExpr: string, prefix: string): string | null {
  const trimmed = typeExpr.trim();
  if (trimmed.startsWith(`${prefix}<`) && trimmed.endsWith(">")) {
    return trimmed.slice(prefix.length + 1, -1).trim();
  }
  return null;
}

function isManifestEntryReadOnly(entry: ManifestEntrypoint, isViewSection: boolean): boolean {
  if (typeof entry.readOnly === "boolean") {
    return entry.readOnly;
  }
  if (isViewSection) {
    return true;
  }
  const noAuth = (entry.authorization ?? "").toLowerCase() === "none";
  return (
    noAuth &&
    (entry.name.startsWith("get_") ||
      entry.name.startsWith("query_") ||
      entry.name.startsWith("is_") ||
      entry.name.startsWith("has_"))
  );
}

/**
 * Spec-driven Soroban argument encoder and entrypoint metadata registry.
 *
 * Designed to work with any Soroban contract spec (either loaded from a
 * checked-in spec JSON or converted from a Grainlify contract manifest).
 */
export class ContractSpec {
  readonly contractName: string;
  readonly version?: string;
  private readonly entrypoints: Map<string, SpecEntrypoint>;
  private readonly types: Map<string, SpecTypeDefinition>;
  private readonly readOnlyMethods: ReadonlySet<string>;

  constructor(definition: ContractSpecDefinition) {
    this.contractName = definition.contractName;
    this.version = definition.version;
    this.entrypoints = new Map();
    this.types = new Map(Object.entries(definition.types ?? {}));

    const readOnly = new Set<string>();
    for (const ep of definition.entrypoints) {
      this.entrypoints.set(ep.name, ep);
      if (ep.readOnly) {
        readOnly.add(ep.name);
      }
    }
    this.readOnlyMethods = readOnly;
  }

  /**
   * Build a `ContractSpec` directly from a Grainlify contract manifest JSON
   * (e.g. `contracts/program-escrow-manifest.json`).
   */
  static fromManifest(
    manifest: ContractManifest,
    types?: Record<string, SpecTypeDefinition>,
  ): ContractSpec {
    const entrypoints: SpecEntrypoint[] = [];
    const seen = new Set<string>();

    const addEntries = (list: ManifestEntrypoint[] | undefined, isView: boolean) => {
      for (const entry of list ?? []) {
        if (seen.has(entry.name)) continue;
        seen.add(entry.name);
        entrypoints.push({
          name: entry.name,
          description: entry.description,
          readOnly: isManifestEntryReadOnly(entry, isView),
          parameters: entry.parameters ?? [],
        });
      }
    };

    addEntries(manifest.entrypoints.view, true);
    addEntries(manifest.entrypoints.public, false);
    addEntries(manifest.entrypoints.admin, false);

    return new ContractSpec({
      contractName: manifest.contract_name,
      version: manifest.version?.current,
      types,
      entrypoints,
    });
  }

  hasMethod(name: string): boolean {
    return this.entrypoints.has(name);
  }

  getMethodNames(): string[] {
    return Array.from(this.entrypoints.keys());
  }

  getReadOnlyMethods(): ReadonlySet<string> {
    return this.readOnlyMethods;
  }

  isReadOnly(name: string): boolean {
    const ep = this.getMethod(name);
    return ep.readOnly;
  }

  getMethod(name: string): SpecEntrypoint {
    const ep = this.entrypoints.get(name);
    if (!ep) {
      const allowed = this.getMethodNames().join(", ");
      throw new ValidationError(
        `Unknown contract method "${name}" for ${this.contractName}. Allowed methods: ${allowed}`,
        "method",
      );
    }
    return ep;
  }

  /**
   * Encode named arguments for a contract entrypoint into ordered `xdr.ScVal[]`.
   *
   * Throws `ValidationError` listing allowed names if the method is unknown,
   * if unknown argument keys are supplied, or if required (non-Option) arguments
   * are missing.
   */
  encodeArgs(
    method: string,
    namedArgs: Record<string, unknown> = {},
  ): xdr.ScVal[] {
    const ep = this.getMethod(method);
    const allowedNames = ep.parameters.map((p) => p.name);
    const allowedSet = new Set(allowedNames);

    const unknownKeys = Object.keys(namedArgs).filter((k) => !allowedSet.has(k));
    if (unknownKeys.length > 0) {
      const allowedDesc =
        allowedNames.length > 0 ? allowedNames.join(", ") : "(none)";
      throw new ValidationError(
        `Unknown argument(s) ${unknownKeys.map((k) => `"${k}"`).join(", ")} for method "${method}". Allowed arguments: ${allowedDesc}`,
        unknownKeys[0],
      );
    }

    const missingKeys = ep.parameters
      .filter((p) => !isOptionType(p.type) && namedArgs[p.name] === undefined)
      .map((p) => p.name);

    if (missingKeys.length > 0) {
      throw new ValidationError(
        `Missing required argument(s) ${missingKeys.map((k) => `"${k}"`).join(", ")} for method "${method}". Allowed arguments: ${allowedNames.join(", ")}`,
        missingKeys[0],
      );
    }

    return ep.parameters.map((param) =>
      this.encodeValue(namedArgs[param.name], param.type, param.name),
    );
  }

  /**
   * Encode a single JavaScript/TypeScript value into a Soroban `xdr.ScVal`
   * according to the given Soroban type expression.
   */
  encodeValue(
    value: unknown,
    typeExpr: string,
    fieldPath: string = "value",
  ): xdr.ScVal {
    const trimmed = typeExpr.trim();

    // Option<T>
    const optionInner = unwrapGeneric(trimmed, "Option");
    if (optionInner !== null) {
      if (value === null || value === undefined) {
        return xdr.ScVal.scvVoid();
      }
      return this.encodeValue(value, optionInner, fieldPath);
    }

    if (value === null || value === undefined) {
      throw new ValidationError(
        `Expected value of type "${trimmed}" for "${fieldPath}", got ${String(value)}`,
        fieldPath,
      );
    }

    // Vec<T>
    const vecInner = unwrapGeneric(trimmed, "Vec");
    if (vecInner !== null) {
      if (!Array.isArray(value)) {
        throw new ValidationError(
          `Expected an array for "${fieldPath}" (${trimmed})`,
          fieldPath,
        );
      }
      return xdr.ScVal.scvVec(
        value.map((item, idx) =>
          this.encodeValue(item, vecInner, `${fieldPath}[${idx}]`),
        ),
      );
    }

    // Map<K, V>
    const mapInner = unwrapGeneric(trimmed, "Map");
    if (mapInner !== null) {
      const [keyType, valType] = splitTopLevelTypeArgs(mapInner);
      if (!keyType || !valType) {
        throw new ValidationError(
          `Invalid Map type expression "${trimmed}" for "${fieldPath}"`,
          fieldPath,
        );
      }
      const entries =
        value instanceof Map
          ? Array.from(value.entries())
          : typeof value === "object"
            ? Object.entries(value as Record<string, unknown>)
            : null;
      if (!entries) {
        throw new ValidationError(
          `Expected a Map or object for "${fieldPath}" (${trimmed})`,
          fieldPath,
        );
      }
      return xdr.ScVal.scvMap(
        entries.map(
          ([k, v]) =>
            new xdr.ScMapEntry({
              key: this.encodeValue(k, keyType, `${fieldPath}.key`),
              val: this.encodeValue(v, valType, `${fieldPath}[${String(k)}]`),
            }),
        ),
      );
    }

    // BytesN<N>
    const bytesNInner = unwrapGeneric(trimmed, "BytesN");
    if (bytesNInner !== null) {
      const expectedLen = Number(bytesNInner);
      const buf = this.toBuffer(value, fieldPath);
      if (!Number.isNaN(expectedLen) && buf.length !== expectedLen) {
        throw new ValidationError(
          `Expected ${expectedLen} bytes for "${fieldPath}" (${trimmed}), got ${buf.length}`,
          fieldPath,
        );
      }
      return nativeToScVal(buf, { type: "bytes" });
    }

    // Primitive types
    switch (trimmed) {
      case "Address": {
        if (typeof value !== "string") {
          throw new ValidationError(
            `Expected Address string for "${fieldPath}"`,
            fieldPath,
          );
        }
        try {
          return Address.fromString(value).toScVal();
        } catch {
          throw new ValidationError(
            `Invalid Stellar address for "${fieldPath}": ${value}`,
            fieldPath,
          );
        }
      }
      case "String": {
        if (typeof value !== "string") {
          throw new ValidationError(
            `Expected string for "${fieldPath}"`,
            fieldPath,
          );
        }
        return nativeToScVal(value, { type: "string" });
      }
      case "Symbol": {
        if (typeof value !== "string") {
          throw new ValidationError(
            `Expected symbol string for "${fieldPath}"`,
            fieldPath,
          );
        }
        return nativeToScVal(value, { type: "symbol" });
      }
      case "bool": {
        if (typeof value !== "boolean") {
          throw new ValidationError(
            `Expected boolean for "${fieldPath}"`,
            fieldPath,
          );
        }
        return nativeToScVal(value, { type: "bool" });
      }
      case "i128":
      case "u128":
      case "i64":
      case "u64": {
        const big = this.toBigInt(value, trimmed, fieldPath);
        return nativeToScVal(big, { type: trimmed });
      }
      case "i32":
      case "u32": {
        const num = this.toInt32(value, trimmed, fieldPath);
        return nativeToScVal(num, { type: trimmed });
      }
      case "Bytes": {
        const buf = this.toBuffer(value, fieldPath);
        return nativeToScVal(buf, { type: "bytes" });
      }
      case "void":
      case "()": {
        return xdr.ScVal.scvVoid();
      }
    }

    // Custom struct or enum defined in the spec
    const customType = this.types.get(trimmed);
    if (customType) {
      return this.encodeCustomType(value, trimmed, customType, fieldPath);
    }

    throw new ValidationError(
      `Unsupported Soroban type "${trimmed}" for "${fieldPath}"`,
      fieldPath,
    );
  }

  private encodeCustomType(
    value: unknown,
    typeName: string,
    def: SpecTypeDefinition,
    fieldPath: string,
  ): xdr.ScVal {
    if (def.kind === "struct") {
      if (typeof value !== "object" || value === null || Array.isArray(value)) {
        throw new ValidationError(
          `Expected object for struct "${typeName}" at "${fieldPath}"`,
          fieldPath,
        );
      }
      const record = value as Record<string, unknown>;
      const allowedFields = def.fields.map((f) => f.name);
      const allowedSet = new Set(allowedFields);

      const unknownFields = Object.keys(record).filter((k) => !allowedSet.has(k));
      if (unknownFields.length > 0) {
        throw new ValidationError(
          `Unknown field(s) ${unknownFields.map((k) => `"${k}"`).join(", ")} for struct "${typeName}" at "${fieldPath}". Allowed fields: ${allowedFields.join(", ")}`,
          `${fieldPath}.${unknownFields[0]}`,
        );
      }

      // Soroban encodes struct fields as ScVal::Map with Symbol keys sorted lexicographically.
      const sortedFields = [...def.fields].sort((a, b) =>
        a.name < b.name ? -1 : a.name > b.name ? 1 : 0,
      );

      const entries = sortedFields.map((field) => {
        const fieldVal = record[field.name];
        if (!isOptionType(field.type) && fieldVal === undefined) {
          throw new ValidationError(
            `Missing required field "${field.name}" for struct "${typeName}" at "${fieldPath}"`,
            `${fieldPath}.${field.name}`,
          );
        }
        return new xdr.ScMapEntry({
          key: xdr.ScVal.scvSymbol(field.name),
          val: this.encodeValue(
            fieldVal,
            field.type,
            `${fieldPath}.${field.name}`,
          ),
        });
      });

      return xdr.ScVal.scvMap(entries);
    }

    if (def.kind === "u32Enum") {
      if (typeof value === "number" && Number.isInteger(value) && value >= 0) {
        const validNums = Object.values(def.variants);
        if (!validNums.includes(value)) {
          throw new ValidationError(
            `Invalid variant value ${value} for enum "${typeName}" at "${fieldPath}". Allowed variants: ${Object.keys(def.variants).join(", ")}`,
            fieldPath,
          );
        }
        return xdr.ScVal.scvU32(value);
      }
      if (typeof value === "string" && value in def.variants) {
        return xdr.ScVal.scvU32(def.variants[value]);
      }
      throw new ValidationError(
        `Invalid variant "${String(value)}" for enum "${typeName}" at "${fieldPath}". Allowed variants: ${Object.keys(def.variants).join(", ")}`,
        fieldPath,
      );
    }

    // Tagged / unit enum -> ScVal::Vec([ScVal::Symbol(tag), ...encodedFields])
    const tag =
      typeof value === "string"
        ? value
        : typeof value === "object" &&
            value !== null &&
            "tag" in value &&
            typeof (value as { tag: unknown }).tag === "string"
          ? (value as { tag: string }).tag
          : null;

    if (!tag) {
      throw new ValidationError(
        `Expected enum variant name or { tag, values? } for "${typeName}" at "${fieldPath}". Allowed variants: ${def.variants.map((v) => v.name).join(", ")}`,
        fieldPath,
      );
    }

    const variant = def.variants.find((v) => v.name === tag);
    if (!variant) {
      throw new ValidationError(
        `Unknown variant "${tag}" for enum "${typeName}" at "${fieldPath}". Allowed variants: ${def.variants.map((v) => v.name).join(", ")}`,
        fieldPath,
      );
    }

    const expectedFields = variant.fields ?? [];
    const providedValues =
      typeof value === "object" &&
      value !== null &&
      "values" in value &&
      Array.isArray((value as { values?: unknown[] }).values)
        ? ((value as { values: unknown[] }).values)
        : [];

    if (providedValues.length !== expectedFields.length) {
      throw new ValidationError(
        `Variant "${tag}" of enum "${typeName}" expects ${expectedFields.length} value(s), got ${providedValues.length}`,
        fieldPath,
      );
    }

    const encodedElements: xdr.ScVal[] = [
      xdr.ScVal.scvSymbol(variant.name),
      ...expectedFields.map((ft, idx) =>
        this.encodeValue(
          providedValues[idx],
          ft,
          `${fieldPath}.${variant.name}[${idx}]`,
        ),
      ),
    ];

    return xdr.ScVal.scvVec(encodedElements);
  }

  private toBigInt(value: unknown, typeName: string, fieldPath: string): bigint {
    if (typeof value === "bigint") return value;
    if (typeof value === "number" && Number.isInteger(value)) {
      return BigInt(value);
    }
    if (typeof value === "string" && /^-?\d+$/.test(value.trim())) {
      return BigInt(value.trim());
    }
    throw new ValidationError(
      `Expected ${typeName} integer for "${fieldPath}", got ${typeof value}`,
      fieldPath,
    );
  }

  private toInt32(value: unknown, typeName: string, fieldPath: string): number {
    const num =
      typeof value === "bigint"
        ? Number(value)
        : typeof value === "number"
          ? value
          : typeof value === "string" && /^-?\d+$/.test(value.trim())
            ? Number(value.trim())
            : NaN;

    if (!Number.isInteger(num)) {
      throw new ValidationError(
        `Expected ${typeName} integer for "${fieldPath}"`,
        fieldPath,
      );
    }
    if (typeName === "u32" && (num < 0 || num > 0xffffffff)) {
      throw new ValidationError(
        `Value ${num} out of range for u32 at "${fieldPath}"`,
        fieldPath,
      );
    }
    if (typeName === "i32" && (num < -0x80000000 || num > 0x7fffffff)) {
      throw new ValidationError(
        `Value ${num} out of range for i32 at "${fieldPath}"`,
        fieldPath,
      );
    }
    return num;
  }

  private toBuffer(value: unknown, fieldPath: string): Buffer {
    if (Buffer.isBuffer(value)) return value;
    if (value instanceof Uint8Array) return Buffer.from(value);
    if (typeof value === "string") {
      const hex = value.startsWith("0x") ? value.slice(2) : value;
      if (/^[0-9a-fA-F]*$/.test(hex) && hex.length % 2 === 0) {
        return Buffer.from(hex, "hex");
      }
    }
    throw new ValidationError(
      `Expected Buffer, Uint8Array, or hex string for "${fieldPath}"`,
      fieldPath,
    );
  }
}

/**
 * Default spec for `ProgramEscrowContract`, loaded from the checked-in spec JSON.
 */
export const PROGRAM_ESCROW_SPEC = new ContractSpec(
  programEscrowSpecJson as ContractSpecDefinition,
);

/**
 * Minimal SEP-41 token contract spec used for token transfers (e.g. `fundContract`).
 */
export const SEP41_TOKEN_SPEC = new ContractSpec({
  contractName: "Sep41TokenContract",
  entrypoints: [
    {
      name: "transfer",
      description: "Transfer tokens from one address to another.",
      readOnly: false,
      parameters: [
        { name: "from", type: "Address" },
        { name: "to", type: "Address" },
        { name: "amount", type: "i128" },
      ],
    },
  ],
});
