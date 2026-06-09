import React from 'react';

interface Column<T> {
  key: string;
  header: React.ReactNode;
  render?: (item: T) => React.ReactNode;
}

interface DataTableProps<T> {
  data: T[];
  columns: Column<T>[];
  keyExtractor: (item: T, index: number) => string;
  className?: string;
}

export function DataTable<T>({ data, columns, keyExtractor, className = '' }: DataTableProps<T>) {
  return (
    <table className={`data-table ${className}`.trim()}>
      <thead>
        <tr>
          {columns.map((col) => (
            <th key={col.key}>{col.header}</th>
          ))}
        </tr>
      </thead>
      <tbody>
        {data.map((item, index) => (
          <tr key={keyExtractor(item, index)}>
            {columns.map((col) => (
              <td key={col.key}>
                {col.render ? col.render(item) : (item as any)[col.key]}
              </td>
            ))}
          </tr>
        ))}
        {data.length === 0 && (
          <tr>
            <td colSpan={columns.length} style={{ textAlign: 'center', color: 'var(--text-low)', padding: '32px' }}>
              No data
            </td>
          </tr>
        )}
      </tbody>
    </table>
  );
}
