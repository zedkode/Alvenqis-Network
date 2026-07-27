import { Link } from "../lib/router";
import { PageHeader } from "../components/PageHeader";

export function NotFoundPage() {
  return (
    <>
      <PageHeader
        title="Not Found"
        description="The requested explorer route does not exist."
      />
      <section className="panel empty-box">
        <p>This page is not available. Return to the local explorer dashboard.</p>
        <Link to="/dashboard">Open dashboard</Link>
      </section>
    </>
  );
}
