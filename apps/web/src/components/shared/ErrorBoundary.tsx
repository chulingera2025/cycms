import { Component, type ErrorInfo, type ReactNode } from 'react';
import { Button, Result } from 'antd';

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('ErrorBoundary caught:', error, info);
  }

  render() {
    if (this.state.error) {
      return (
        this.props.fallback ?? (
          <div className="grid min-h-screen place-items-center p-6">
            <Result
              status="error"
              title="出现错误"
              subTitle={this.state.error.message}
              extra={[
                <Button
                  key="retry"
                  type="primary"
                  onClick={() => this.setState({ error: null })}
                >
                  重试
                </Button>,
                <Button key="home" onClick={() => window.location.assign('/')}>
                  回首页
                </Button>,
              ]}
            />
          </div>
        )
      );
    }
    return this.props.children;
  }
}
