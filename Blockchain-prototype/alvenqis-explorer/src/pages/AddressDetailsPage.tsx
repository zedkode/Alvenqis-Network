import { useEffect, useState } from "react";
import { Link, useParams } from "../lib/router";
import { AddressActivity, fetchJson } from "../lib/api";
import { formatAtomic } from "../lib/format";
import { PageHeader } from "../components/PageHeader";
import { ErrorPanel, LoadingPanel } from "../components/StatePanels";

export function AddressDetailsPage() {
  const { address } = useParams();
  const [activity, setActivity] = useState<AddressActivity | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    if (!address) {
      setError("Address is required.");
      return;
    }

    fetchJson<AddressActivity>(`/indexer/address/${address}`)
      .then((loaded) => {
        if (active) {
          setActivity(loaded);
        }
      })
      .catch((loadError) => {
        if (active) {
          setError(loadError instanceof Error ? loadError.message : "Failed to load address");
        }
      });

    return () => {
      active = false;
    };
  }, [address]);

  return (
    <>
      <PageHeader
        title="Address Details"
        description="Local balance snapshot and indexed activity for one Alvenqis network address."
      />
      {error ? <ErrorPanel message={error} /> : null}
      {!activity && !error ? <LoadingPanel message="Loading indexed address activity..." /> : null}
      {activity ? (
        <div className="grid two-col">
          <section className="panel">
            <h2>Address State</h2>
            <div className="detail-list">
              <div className="detail-row">
                <div className="detail-label">Address</div>
                <div className="hash-text">{activity.address}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Exists in ledger</div>
                <div>{String(activity.exists_in_ledger)}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Balance</div>
                <div>{formatAtomic(activity.balance_atomic)}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Total received</div>
                <div>{formatAtomic(activity.total_received_atomic)}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Total sent</div>
                <div>{formatAtomic(activity.total_sent_atomic)}</div>
              </div>
              <div className="detail-row">
                <div className="detail-label">Mined reward</div>
                <div>{formatAtomic(activity.mined_reward_atomic)}</div>
              </div>
            </div>
          </section>

          <section className="panel">
            <h2>Activity</h2>
            <div className="activity-list">
              <div>
                <strong>Transactions</strong>
                <div className="tx-list">
                  {activity.transaction_hashes.map((hash) => (
                    <div className="tx-pill" key={hash}>
                      <Link className="hash-text" to={`/tx/${hash}`}>
                        {hash}
                      </Link>
                    </div>
                  ))}
                </div>
              </div>
              <div>
                <strong>Mined block heights</strong>
                <div className="tx-list">
                  {activity.mined_block_heights.map((height) => (
                    <div className="tx-pill" key={height}>
                      <Link to={`/blocks/${height}`}>Block {height}</Link>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          </section>
        </div>
      ) : null}
    </>
  );
}
