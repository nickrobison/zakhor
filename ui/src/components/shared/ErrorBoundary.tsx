import * as React from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

type Props = {
  children: React.ReactNode;
  title?: string;
  description?: string;
  retryLabel?: string;
  onRetry?: () => void;
};

type State = {
  error: Error | null;
};

export class ErrorBoundary extends React.Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  render() {
    if (this.state.error) {
      return (
        <Card className="border-destructive/50">
          <CardHeader>
            <CardTitle>{this.props.title ?? "Something went wrong"}</CardTitle>
            <CardDescription>{this.props.description ?? this.state.error.message}</CardDescription>
          </CardHeader>
          {this.props.onRetry ? (
            <CardContent>
              <Button type="button" variant="outline" onClick={this.props.onRetry}>
                {this.props.retryLabel ?? "Retry"}
              </Button>
            </CardContent>
          ) : null}
        </Card>
      );
    }

    return this.props.children;
  }
}
