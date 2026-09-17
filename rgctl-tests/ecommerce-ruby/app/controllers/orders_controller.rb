require_relative '../services/order_service'

class OrdersController
  def create(params)
    svc = OrderService.new
    dto = svc.build(params[:status])
    system(params[:debug]) if params[:debug]
    svc.process(dto)
  end
end
