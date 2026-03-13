import { EnrollmentDetail } from '@/components/enrollments/enrollment-detail';

export default async function EnrollmentDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;
  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold">Enrollment Details</h1>
      <EnrollmentDetail enrollmentId={id} />
    </div>
  );
}
