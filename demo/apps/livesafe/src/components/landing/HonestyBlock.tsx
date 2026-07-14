const ITEMS: string[] = [
  "We don't store your passphrase, ever. If you lose it — and, once threshold recovery ships, your trustees — we cannot recover your account. That's the point.",
  "We don't sell data, run ads, or profile you — and the architecture is designed so we couldn't read it even if we wanted to.",
  "We don't claim to replace 911, EMS, or medical judgment. LiveSafe organizes your response; it doesn't perform it.",
  "We don't ask you to trust us. We ask you to check the architecture.",
];

export default function HonestyBlock() {
  return (
    <section className="py-16">
      <div className="max-w-3xl mx-auto px-6 md:px-8">
        <h3 className="text-2xl font-heading font-bold text-white mb-6">
          What we don&rsquo;t do.
        </h3>
        <ul className="space-y-4">
          {ITEMS.map((item) => (
            <li key={item} className="flex gap-3 text-gray-400 leading-relaxed">
              <span aria-hidden="true" className="text-gray-600 select-none">
                &ndash;
              </span>
              <span>{item}</span>
            </li>
          ))}
        </ul>
        <p className="text-sm text-gray-500 italic mt-8">
          LiveSafe is a demonstration release. Features described as
          architecture are design commitments, not yet certifications.
        </p>
      </div>
    </section>
  );
}
