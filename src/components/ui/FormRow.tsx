import React from 'react';

interface FormRowProps {
  label: React.ReactNode;
  description?: React.ReactNode;
  children: React.ReactNode;
  className?: string;
}

export function FormRow({ label, description, children, className = '' }: FormRowProps) {
  return (
    <div className={`form-row ${className}`.trim()}>
      <div className="form-row-label">
        <h4>{label}</h4>
        {description && <p>{description}</p>}
      </div>
      <div className="form-row-control">
        {children}
      </div>
    </div>
  );
}
