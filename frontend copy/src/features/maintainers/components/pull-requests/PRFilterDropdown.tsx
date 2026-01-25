import { PRFilterType } from '../../types';
import { GlassDropdown } from '../../../../shared/components';

interface PRFilterDropdownProps {
  value: PRFilterType;
  onChange: (value: PRFilterType) => void;
  isOpen: boolean;
  onToggle: () => void;
  onClose: () => void;
}

const filterOptions: PRFilterType[] = ['All states', 'Open', 'Merged', 'Closed', 'Draft'];

export function PRFilterDropdown({ value, onChange, isOpen, onToggle, onClose }: PRFilterDropdownProps) {
  return (
    <GlassDropdown
      value={value}
      onChange={onChange}
      options={filterOptions}
      isOpen={isOpen}
      onToggle={onToggle}
      onClose={onClose}
    />
  );
}