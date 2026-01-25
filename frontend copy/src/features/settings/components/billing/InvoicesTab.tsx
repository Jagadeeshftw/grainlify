import { Download, FileText, CheckCircle2, Clock, AlertCircle } from 'lucide-react';
import { useTheme } from '../../../../shared/contexts/ThemeContext';
import { Invoice, InvoiceStatus } from '../../types';

interface InvoicesTabProps {
  invoices: Invoice[];
}

export function InvoicesTab({ invoices }: InvoicesTabProps) {
  const { theme } = useTheme();

  const getStatusColor = (status: InvoiceStatus) => {
    switch (status) {
      case 'paid':
        return {
          bg: 'bg-[#22c55e]/20',
          text: 'text-[#16a34a]',
          border: 'border-[#22c55e]/30',
          icon: CheckCircle2,
        };
      case 'pending':
        return {
          bg: 'bg-[#eab308]/20',
          text: 'text-[#ca8a04]',
          border: 'border-[#eab308]/30',
          icon: Clock,
        };
      case 'overdue':
        return {
          bg: 'bg-[#ef4444]/20',
          text: 'text-[#dc2626]',
          border: 'border-[#ef4444]/30',
          icon: AlertCircle,
        };
    }
  };

  const handleDownloadInvoice = (invoice: Invoice) => {
    // In a real app, this would trigger a PDF download
    console.log('Downloading invoice:', invoice.invoiceNumber);
    // Simulate download
    const link = document.createElement('a');
    link.href = '#'; // In real app, this would be the PDF URL
    link.download = `invoice-${invoice.invoiceNumber}.pdf`;
    link.click();
  };

  return (
    <div className={`backdrop-blur-[40px] rounded-[24px] border shadow-[0_8px_32px_rgba(0,0,0,0.08)] p-8 transition-colors ${
      theme === 'dark'
        ? 'bg-[#2d2820]/[0.4] border-white/10'
        : 'bg-white/[0.12] border-white/20'
    }`}>
      {/* Header */}
      <div className="mb-6">
        <h3 className={`text-[20px] font-bold mb-2 transition-colors ${
          theme === 'dark' ? 'text-[#f5efe5]' : 'text-[#2d2820]'
        }`}>Invoices</h3>
        <p className={`text-[14px] transition-colors ${
          theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
        }`}>
          View and download your billing invoices.
        </p>
      </div>

      {/* Invoices List */}
      {invoices.length > 0 ? (
        <div className="space-y-3">
          {/* Table Header */}
          <div className={`grid grid-cols-[1.2fr_1fr_1fr_1fr_0.8fr_0.6fr] gap-4 px-6 py-3 border-b-2 transition-colors ${
            theme === 'dark' ? 'border-white/20' : 'border-white/20'
          }`}>
            <div className={`text-[12px] font-bold uppercase tracking-wide transition-colors ${
              theme === 'dark' ? 'text-[#d4c5b0]' : 'text-[#7a6b5a]'
            }`}>Invoice</div>
            <div className={`text-[12px] font-bold uppercase tracking-wide transition-colors ${
              theme === 'dark' ? 'text-[#d4c5b0]' : 'text-[#7a6b5a]'
            }`}>Date</div>
            <div className={`text-[12px] font-bold uppercase tracking-wide transition-colors ${
              theme === 'dark' ? 'text-[#d4c5b0]' : 'text-[#7a6b5a]'
            }`}>Amount</div>
            <div className={`text-[12px] font-bold uppercase tracking-wide transition-colors ${
              theme === 'dark' ? 'text-[#d4c5b0]' : 'text-[#7a6b5a]'
            }`}>Period</div>
            <div className={`text-[12px] font-bold uppercase tracking-wide transition-colors ${
              theme === 'dark' ? 'text-[#d4c5b0]' : 'text-[#7a6b5a]'
            }`}>Status</div>
            <div className={`text-[12px] font-bold uppercase tracking-wide transition-colors ${
              theme === 'dark' ? 'text-[#d4c5b0]' : 'text-[#7a6b5a]'
            }`}>Action</div>
          </div>

          {/* Invoice Rows */}
          {invoices.map((invoice) => {
            const statusConfig = getStatusColor(invoice.status);
            const StatusIcon = statusConfig.icon;

            return (
              <div
                key={invoice.id}
                className={`grid grid-cols-[1.2fr_1fr_1fr_1fr_0.8fr_0.6fr] gap-4 px-6 py-5 rounded-[16px] backdrop-blur-[25px] border transition-all ${
                  theme === 'dark'
                    ? 'bg-white/[0.08] border-white/15 hover:bg-white/[0.12] hover:border-[#c9983a]/20'
                    : 'bg-white/[0.08] border-white/15 hover:bg-white/[0.15] hover:border-[#c9983a]/20'
                }`}
              >
                {/* Invoice Number & Description */}
                <div>
                  <div className="flex items-center gap-2 mb-1">
                    <div className={`w-8 h-8 rounded-[10px] flex items-center justify-center ${
                      theme === 'dark' ? 'bg-[#c9983a]/20' : 'bg-[#c9983a]/15'
                    }`}>
                      <FileText className="w-4 h-4 text-[#c9983a]" />
                    </div>
                    <h4 className={`text-[14px] font-bold transition-colors ${
                      theme === 'dark' ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                    }`}>
                      {invoice.invoiceNumber}
                    </h4>
                  </div>
                  <p className={`text-[12px] ml-10 transition-colors ${
                    theme === 'dark' ? 'text-[#8a7e70]' : 'text-[#9a8b7a]'
                  }`}>
                    {invoice.description}
                  </p>
                </div>

                {/* Date */}
                <div className="flex items-center">
                  <span className={`text-[13px] transition-colors ${
                    theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                  }`}>
                    {new Date(invoice.date).toLocaleDateString('en-US', {
                      month: 'short',
                      day: 'numeric',
                      year: 'numeric'
                    })}
                  </span>
                </div>

                {/* Amount */}
                <div className="flex items-center">
                  <span className={`text-[15px] font-bold transition-colors ${
                    theme === 'dark' ? 'text-[#e8dfd0]' : 'text-[#2d2820]'
                  }`}>
                    {invoice.amount.toLocaleString('en-US', {
                      style: 'currency',
                      currency: invoice.currency
                    })}
                  </span>
                </div>

                {/* Billing Period */}
                <div className="flex items-center">
                  <span className={`text-[13px] transition-colors ${
                    theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
                  }`}>
                    {invoice.billingPeriod}
                  </span>
                </div>

                {/* Status */}
                <div className="flex items-center">
                  <div className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-[8px] border ${statusConfig.bg} ${statusConfig.border}`}>
                    <StatusIcon className={`w-3 h-3 ${statusConfig.text}`} />
                    <span className={`text-[11px] font-semibold capitalize ${statusConfig.text}`}>
                      {invoice.status}
                    </span>
                  </div>
                </div>

                {/* Download Action */}
                <div className="flex items-center">
                  <button
                    onClick={() => handleDownloadInvoice(invoice)}
                    className={`p-2.5 rounded-[10px] transition-all ${
                      theme === 'dark'
                        ? 'hover:bg-white/[0.15] text-[#c9983a]'
                        : 'hover:bg-white/[0.2] text-[#c9983a]'
                    }`}
                    title="Download Invoice"
                  >
                    <Download className="w-4 h-4" />
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      ) : (
        <div className="text-center py-12">
          <div className={`w-16 h-16 rounded-full mx-auto mb-4 flex items-center justify-center ${
            theme === 'dark' ? 'bg-white/[0.08]' : 'bg-white/[0.15]'
          }`}>
            <FileText className={`w-8 h-8 ${
              theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
            }`} />
          </div>
          <p className={`text-[14px] mb-2 transition-colors ${
            theme === 'dark' ? 'text-[#b8a898]' : 'text-[#7a6b5a]'
          }`}>
            No invoices yet
          </p>
          <p className={`text-[13px] transition-colors ${
            theme === 'dark' ? 'text-[#8a7e70]' : 'text-[#9a8b7a]'
          }`}>
            Your billing invoices will appear here
          </p>
        </div>
      )}
    </div>
  );
}
