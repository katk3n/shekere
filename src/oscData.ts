export interface OscMonitorData {
  displayText: string;
  keyValueArgs?: Record<string, string>;
}

function unwrapOscArgument(argument: unknown): unknown {
  if (typeof argument === "object" && argument !== null) {
    const values = Object.values(argument);
    return values.length > 0 ? values[0] : JSON.stringify(argument);
  }
  return argument;
}

function formatOscArgument(argument: unknown): string {
  if (typeof argument === "number") {
    return Number.isInteger(argument) ? argument.toString() : argument.toFixed(2);
  }
  return String(argument);
}

export function parseOscMonitorArgs(args: unknown): OscMonitorData {
  if (!Array.isArray(args)) return { displayText: "" };

  const rawArgs = args.map(unwrapOscArgument);
  let isKeyValueFormat = rawArgs.length >= 2 && rawArgs.length % 2 === 0;
  for (let index = 0; index < rawArgs.length; index += 2) {
    if (typeof rawArgs[index] !== "string") {
      isKeyValueFormat = false;
      break;
    }
  }

  if (isKeyValueFormat) {
    const keyValueArgs: Record<string, string> = {};
    const focusKeys = ["s", "n", "cps", "note", "gain", "speed", "vowel"];
    const pairs: string[] = [];
    for (let index = 0; index < rawArgs.length; index += 2) {
      const key = String(rawArgs[index]);
      const value = rawArgs[index + 1];
      keyValueArgs[key] = String(value);
      if (focusKeys.includes(key)) pairs.push(`${key}: ${formatOscArgument(value)}`);
    }

    if (pairs.length === 0) {
      pairs.push(`${rawArgs[0]}: ${rawArgs[1]}`);
      if (rawArgs.length > 2) pairs.push("...");
    }

    return { displayText: pairs.join(", "), keyValueArgs };
  }

  const displayArgs = rawArgs.slice(0, 3).map(formatOscArgument);
  let displayText = displayArgs.join(", ");
  if (rawArgs.length > 3) displayText += ", ...";
  return { displayText };
}

export function convertOscSketchData(address: string, args: readonly unknown[]): unknown {
  if (address !== "/dirt/play" || args.length % 2 !== 0) return args;

  const data: Record<string, unknown> = {};
  for (let index = 0; index < args.length; index += 2) {
    data[String(args[index])] = args[index + 1];
  }
  return data;
}
