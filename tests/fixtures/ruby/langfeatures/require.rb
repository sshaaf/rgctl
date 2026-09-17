require_relative '../models/user'

class Auth
  def login
    User.find(1)
  end
end
