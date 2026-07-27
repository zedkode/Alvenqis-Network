import type { ReactNode } from "react";

export interface DataColumn<T> { key: string; label: string; render(row: T): ReactNode; }

export function DataTable<T>({ columns, rows, rowKey, onRow }: { columns: DataColumn<T>[]; rows: T[]; rowKey(row: T): string; onRow?(row: T): void }) {
  return <div className="data-table-shell"><table className="data-table"><thead><tr>{columns.map((column) => <th key={column.key}>{column.label}</th>)}</tr></thead><tbody>{rows.map((row) => <tr key={rowKey(row)} onClick={() => onRow?.(row)}>{columns.map((column) => <td key={column.key}>{column.render(row)}</td>)}</tr>)}</tbody></table></div>;
}
