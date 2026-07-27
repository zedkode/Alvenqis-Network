import { Link } from "react-router-dom";
import { PageHeader } from "../components/PageHeader";

export function NotFoundPage() {
  return (
    <>
      <PageHeader
        title="Not Found"
        description="The requested explorer route does not exist in this local draft UI."
      />
      <section className="panel empty-box">
        <p>This page is not available. Return to the local explorer dashboard.</p>
        <Link to="/dashboard">Open dashboard</Link>
      </section>
    </>
  );
}
